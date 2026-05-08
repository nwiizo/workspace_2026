# Lambda invoke smoke test

`lambda_function.py` is a minimal Python echo function. Zip it and deploy
to the emulator under test.

```sh
zip function.zip lambda_function.py

# Either emulator: register a role first so CreateFunction accepts --role.
aws --endpoint-url http://localhost:4566 iam create-role \
  --role-name echo-role \
  --assume-role-policy-document '{"Version":"2012-10-17","Statement":[{"Effect":"Allow","Principal":{"Service":"lambda.amazonaws.com"},"Action":"sts:AssumeRole"}]}'

ROLE=$(aws --endpoint-url http://localhost:4566 iam get-role --role-name echo-role --query 'Role.Arn' --output text)

aws --endpoint-url http://localhost:4566 lambda create-function \
  --function-name echo --runtime python3.12 --role "$ROLE" \
  --handler lambda_function.lambda_handler --zip-file fileb://function.zip

aws --endpoint-url http://localhost:4566 lambda invoke \
  --function-name echo --payload '{"k":"v"}' \
  --cli-binary-format raw-in-base64-out out.json
cat out.json
```

## Required container setup

Both emulators need a live Docker daemon to actually invoke.

- **rustack**: start with `-e LAMBDA_DOCKER_ENABLED=true -v /var/run/docker.sock:/var/run/docker.sock`.
- **fakecloud**: the published `ghcr.io/faiscadev/fakecloud:latest` image has no `docker`/`podman` CLI inside, so `lambda invoke` fails with `ServiceException: Docker/Podman is required` even when the host socket is mounted. Run the native binary on the host or build a derived image with the Docker CLI added.
