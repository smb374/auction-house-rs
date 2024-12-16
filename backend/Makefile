.PHONY: build clean deploy

build:
	cargo lambda build --release

clean:
	cargo clean

deploy: build
	rm -f ./bootstrap.zip
	terraform apply
