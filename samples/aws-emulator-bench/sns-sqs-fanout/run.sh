#!/usr/bin/env bash
# SNS -> SQS fanout smoke test against an AWS emulator.
# Usage: ./run.sh http://localhost:4566
set -euo pipefail
EP="${1:-http://localhost:4566}"
export AWS_ACCESS_KEY_ID=test AWS_SECRET_ACCESS_KEY=test AWS_REGION=us-east-1

TOPIC=$(aws --endpoint-url "$EP" sns create-topic --name bench-topic --query TopicArn --output text)
QURL=$(aws --endpoint-url "$EP" sqs create-queue --queue-name bench-queue --query QueueUrl --output text)
QARN=$(aws --endpoint-url "$EP" sqs get-queue-attributes --queue-url "$QURL" \
  --attribute-names QueueArn --query 'Attributes.QueueArn' --output text)
SUB=$(aws --endpoint-url "$EP" sns subscribe --topic-arn "$TOPIC" --protocol sqs \
  --notification-endpoint "$QARN" --query SubscriptionArn --output text)
aws --endpoint-url "$EP" sns set-subscription-attributes \
  --subscription-arn "$SUB" --attribute-name RawMessageDelivery --attribute-value true >/dev/null
aws --endpoint-url "$EP" sns publish --topic-arn "$TOPIC" --message "fanout-ok" >/dev/null
sleep 0.5
aws --endpoint-url "$EP" sqs receive-message --queue-url "$QURL" --wait-time-seconds 2
