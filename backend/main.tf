terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
    random = {
      source  = "hashicorp/random"
      version = "~> 3.6"
    }
    archive = {
      source  = "hashicorp/archive"
      version = "~> 2.7"
    }
  }
  required_version = "~> 1.0"
}

provider "aws" {
  shared_config_files      = ["~/.aws/config"]
  shared_credentials_files = ["~/.aws/credentials"]
  profile                  = "terraform"
}

# Lambda function
resource "random_pet" "lambda_bucket_name" {
  prefix = "auction-house-rs"
  length = 4
}

resource "aws_s3_bucket" "lambda_bucket" {
  bucket        = random_pet.lambda_bucket_name.id
  force_destroy = true
}

resource "aws_s3_bucket_ownership_controls" "lambda_bucket" {
  bucket = aws_s3_bucket.lambda_bucket.id

  rule {
    object_ownership = "ObjectWriter"
  }
}

resource "aws_s3_bucket_acl" "lambda_bucket" {
  bucket     = aws_s3_bucket.lambda_bucket.id
  acl        = "private"
  depends_on = [aws_s3_bucket_ownership_controls.lambda_bucket]
}

resource "aws_s3_bucket_versioning" "lambda_bucket" {
  bucket = aws_s3_bucket.lambda_bucket.id

  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "lambda_bucket" {
  bucket = aws_s3_bucket.lambda_bucket.id

  rule {
    id     = "Retain5Versions"
    status = "Enabled"

    noncurrent_version_expiration {
      noncurrent_days           = 1
      newer_noncurrent_versions = "5"
    }
  }
}

data "archive_file" "lambda_auction_house" {
  type = "zip"

  source_file = "${path.module}/target/lambda/auction-house-rs/bootstrap"
  output_path = "${path.module}/bootstrap.zip"
}

resource "aws_s3_object" "lambda_auction_house" {
  bucket = aws_s3_bucket.lambda_bucket.id

  key    = "bootstrap.zip"
  source = data.archive_file.lambda_auction_house.output_path

  etag = filemd5(data.archive_file.lambda_auction_house.output_path)
}

variable "JWT_SECRET" {
  type        = string
  description = "JWT secret for signing JWT tokens"
  sensitive   = true
}

resource "aws_lambda_function" "auction_house" {
  function_name = "AuctionHouse"

  s3_bucket = aws_s3_bucket.lambda_bucket.id
  s3_key    = aws_s3_object.lambda_auction_house.key

  runtime = "provided.al2023"
  handler = "bootstrap"

  source_code_hash = data.archive_file.lambda_auction_house.output_base64sha256

  role    = aws_iam_role.lambda_execution.arn
  publish = true

  environment {
    variables = {
      JWT_SECRET = var.JWT_SECRET
    }
  }
}

resource "aws_cloudwatch_log_group" "auction_house" {
  name = "/aws/lambda/${aws_lambda_function.auction_house.function_name}"

  retention_in_days = 7
}

# API Gateway
resource "aws_api_gateway_account" "api_access_logs" {
  cloudwatch_role_arn = aws_iam_role.cloudwatch.arn
}

data "aws_iam_policy_document" "assume_role" {
  statement {
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["apigateway.amazonaws.com"]
    }

    actions = ["sts:AssumeRole"]
  }
}

resource "aws_iam_role" "cloudwatch" {
  name               = "api_gateway_cloudwatch"
  assume_role_policy = data.aws_iam_policy_document.assume_role.json
}

data "aws_iam_policy_document" "cloudwatch" {
  statement {
    effect = "Allow"

    actions = [
      "logs:CreateLogGroup",
      "logs:CreateLogStream",
      "logs:DescribeLogGroups",
      "logs:DescribeLogStreams",
      "logs:PutLogEvents",
      "logs:GetLogEvents",
      "logs:FilterLogEvents",
    ]

    resources = ["*"]
  }
}
resource "aws_iam_role_policy" "cloudwatch" {
  name   = "default"
  role   = aws_iam_role.cloudwatch.id
  policy = data.aws_iam_policy_document.cloudwatch.json
}

resource "aws_apigatewayv2_api" "auction_house" {
  name          = "auction-house-rs"
  protocol_type = "HTTP"
  cors_configuration {
    allow_origins = ["*"]
    allow_methods = [
      "OPTIONS",
      "GET",
      "POST",
      "PUT",
      "PATCH",
      "HEAD",
      "DELETE"
    ]
    allow_headers = [
      "content-type",
      "authorization",
      "x-amz-date",
      "x-api-key",
      "x-amz-security-token",
      "x-amz-user-agent",
      "x-amzn-trace-id"
    ]
    max_age           = 0
    allow_credentials = false
  }
}

resource "aws_apigatewayv2_integration" "auction_house" {
  api_id                 = aws_apigatewayv2_api.auction_house.id
  integration_type       = "AWS_PROXY"
  connection_type        = "INTERNET"
  integration_method     = "POST"
  timeout_milliseconds   = 30000
  payload_format_version = "2.0"
  integration_uri        = aws_lambda_function.auction_house.invoke_arn
}

resource "aws_apigatewayv2_route" "auction_house" {
  api_id    = aws_apigatewayv2_api.auction_house.id
  route_key = "ANY /{proxy+}"

  target = "integrations/${aws_apigatewayv2_integration.auction_house.id}"
}

resource "aws_lambda_permission" "lambda_api_invoke" {
  statement_id  = "AllowExecutionFromAPIGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.auction_house.function_name
  principal     = "apigateway.amazonaws.com"

  source_arn = "${aws_apigatewayv2_api.auction_house.execution_arn}/*"
}

resource "aws_cloudwatch_log_group" "api_access_logs" {
  name              = "/aws/apigateway/${aws_apigatewayv2_api.auction_house.name}"
  retention_in_days = 7
}

resource "aws_apigatewayv2_stage" "auction_house_stage" {
  api_id      = aws_apigatewayv2_api.auction_house.id
  name        = "v1"
  auto_deploy = true

  access_log_settings {
    destination_arn = aws_cloudwatch_log_group.api_access_logs.arn
    format = jsonencode({
      requestId          = "$context.requestId",
      ip                 = "$context.identity.sourceIp",
      requestTime        = "$context.requestTime",
      httpMethod         = "$context.httpMethod",
      routeKey           = "$context.routeKey",
      status             = "$context.status",
      protocol           = "$context.protocol",
      responseLength     = "$context.responseLength",
      integrationLatency = "$context.integrationLatency"
    })
  }
}

# S3
resource "aws_s3_bucket" "image_bucket" {
  bucket = "auction-house-rs-images"
}

resource "aws_s3_bucket_cors_configuration" "image_bucket" {
  bucket = aws_s3_bucket.image_bucket.id

  cors_rule {
    allowed_headers = ["*"]
    allowed_methods = ["GET", "PUT", "POST", "DELETE"]
    allowed_origins = ["*"]
    max_age_seconds = 3000
  }
}

# DynamoDB
resource "aws_dynamodb_table" "seller_table" {
  name         = "auction-sellers"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "id"

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_dynamodb_table" "buyer_table" {
  name         = "auction-buyers"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "id"

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_dynamodb_table" "item_table" {
  name         = "auction-items"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "sellerId"
  range_key    = "id"

  attribute {
    name = "sellerId"
    type = "S"
  }

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_dynamodb_table" "bid_table" {
  name         = "auction-bids"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "buyerId"
  range_key    = "id"

  attribute {
    name = "buyerId"
    type = "S"
  }

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_dynamodb_table" "purchase_table" {
  name         = "auction-purchases"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "buyerId"
  range_key    = "id"

  attribute {
    name = "buyerId"
    type = "S"
  }

  attribute {
    name = "id"
    type = "S"
  }
}

resource "aws_dynamodb_table" "unfreeze_request_table" {
  name         = "auction-unfreeze-requests"
  billing_mode = "PAY_PER_REQUEST"
  hash_key     = "sellerId"
  range_key    = "id"

  attribute {
    name = "sellerId"
    type = "S"
  }

  attribute {
    name = "id"
    type = "S"
  }
}

# IAM Execution Role
data "aws_iam_policy_document" "lambda_execution_policy_spec" {
  statement {
    effect = "Allow"
    actions = [
      "dynamodb:*"
    ]
    resources = [
      aws_dynamodb_table.seller_table.arn,
      aws_dynamodb_table.buyer_table.arn,
      aws_dynamodb_table.item_table.arn,
      aws_dynamodb_table.bid_table.arn,
      aws_dynamodb_table.purchase_table.arn,
      aws_dynamodb_table.unfreeze_request_table.arn,
    ]
  }
  statement {
    effect = "Allow"
    actions = [
      "s3:PutObject",
      "s3:GetObject",
      "s3:DeleteObject"
    ]
    resources = [
      "${aws_s3_bucket.image_bucket.arn}/*"
    ]
  }
  statement {
    effect = "Allow"
    actions = [
      "logs:CreateLogGroup",
      "logs:CreateLogStream",
      "logs:PutLogEvents"
    ]
    resources = ["*"]
  }
}

resource "aws_iam_policy" "lambda_execution_policy" {
  name   = "lambda-execution-policy"
  policy = data.aws_iam_policy_document.lambda_execution_policy_spec.json
}

resource "aws_iam_role" "lambda_execution" {
  name = "lambda-execution-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
        Action = "sts:AssumeRole"
      }
    ]
  })
}

resource "aws_iam_role_policy_attachment" "lambda_execution_attachment" {
  role       = aws_iam_role.lambda_execution.name
  policy_arn = aws_iam_policy.lambda_execution_policy.arn
}

# IAM user
data "aws_iam_policy_document" "lambda_service_policy_spec" {
  statement {
    effect = "Allow"
    actions = [
      "lambda:GetFunction",
      "lambda:GetLayerVersion",
      "lambda:CreateFunction",
      "lambda:UpdateFunctionCode",
      "lambda:UpdateFunctionConfiguration",
      "lambda:PublishVersion",
      "lambda:TagResource"
    ]
    resources = [
      "arn:aws:lambda:*:*:function:*",
    ]
  }
  statement {
    effect = "Allow"
    actions = [
      "iam:PassRole"
    ]
    resources = ["*"]
  }
}

resource "aws_iam_policy" "lambda_service_policy" {
  name   = "lambda-service-policy"
  policy = data.aws_iam_policy_document.lambda_service_policy_spec.json
}

resource "aws_iam_user" "lambda_service_user" {
  name = "lambda-service-user"
}

resource "aws_iam_access_key" "lambda_service_user" {
  user = aws_iam_user.lambda_service_user.name
}

resource "aws_iam_user_policy_attachment" "lambda_service_user_policy_attachment" {
  user       = aws_iam_user.lambda_service_user.name
  policy_arn = aws_iam_policy.lambda_service_policy.arn
}

resource "aws_iam_user_policy_attachment" "lambda_execution_user_policy_attachment" {
  user       = aws_iam_user.lambda_service_user.name
  policy_arn = aws_iam_policy.lambda_execution_policy.arn
}

# Outputs
output "aws_access_key_id" {
  value = aws_iam_access_key.lambda_service_user.id
}

output "aws_secret_access_key" {
  value     = aws_iam_access_key.lambda_service_user.secret
  sensitive = true
}

output "lambda_bucket_name" {
  description = "Name of the S3 bucket used to store function code."

  value = aws_s3_bucket.lambda_bucket.id
}

output "function_name" {
  description = "Name of the Lambda function."

  value = aws_lambda_function.auction_house.function_name
}

output "api_url" {
  value = aws_apigatewayv2_api.auction_house.api_endpoint
}
