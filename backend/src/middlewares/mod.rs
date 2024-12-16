use axum::{extract::Request, middleware::Next, response::IntoResponse};
use lambda_http::{request::RequestContext, tracing};

pub mod auth;

pub async fn trace_client(req: Request, next: Next) -> impl IntoResponse {
    let ctx = req.extensions().get::<RequestContext>();
    if let Some(RequestContext::ApiGatewayV2(v2ctx)) = ctx {
        let http_ctx = &v2ctx.http;
        let source_ip = http_ctx
            .source_ip
            .as_ref()
            .map_or("unknown", |v| v.as_str());
        let path = req.uri().path();

        tracing::info!("{} -> {}", source_ip, path);
    }
    next.run(req).await
}
