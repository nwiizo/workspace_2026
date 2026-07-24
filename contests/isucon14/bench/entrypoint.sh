#!/bin/bash

TASK_METADATA_URI=$ECS_CONTAINER_METADATA_URI_V4/task

ZONE_NAME=$(curl -s $TASK_METADATA_URI | jq -r .AvailabilityZone)
ZONE_ID=$(aws ec2 describe-availability-zones |
	jq -r ".AvailabilityZones[] | select(.ZoneName==\"${ZONE_NAME}\").ZoneId")
export ISUXPORTAL_SUPERVISOR_INSTANCE_NAME="${ZONE_ID}"


# クラスタARNを取得し、クラスタIDを抽出
CLUSTER_ARN=$(curl -s $TASK_METADATA_URI | jq -r '.Cluster')
CLUSTER_ID=$(echo $CLUSTER_ARN | awk -F'/' '{print $2}')

# タスクARNを取得
TASK_ARN=$(curl -s $TASK_METADATA_URI | jq -r '.TaskARN')
TASK_ID=$(echo $TASK_ARN | awk -F'/' '{print $3}')

# ENI (Elastic Network Interface) IDを取得
ENI_ID=$(aws ecs describe-tasks \
  --cluster $CLUSTER_ID \
  --tasks $TASK_ID \
  --query "tasks[0].attachments[0].details[?name=='networkInterfaceId'].value" \
  --output text)

# ENIに関連付けられたパブリックIPを取得
PUBLIC_IP=$(aws ec2 describe-network-interfaces \
  --network-interface-ids $ENI_ID \
  --query "NetworkInterfaces[0].Association.PublicIp" \
  --output text)

exec "$@" --payment-url http://$PUBLIC_IP:12345
