# kuroko

A lightweight AWS service emulator in Rust. Single static binary, MIT licensed,
no authentication, no telemetry.

## Status

- **Framework**: axum 0.8 + tokio, single binary, port `4566` (LocalStack-compatible).
- **Persistence**: optional JSON snapshot per service (atomic rename), enabled
  by `KUROKO_DATA_DIR`. In-memory when unset.
- **Coverage**: 76 AWS services registered.
  - Fully implemented (23): **S3**, **SQS**, **DynamoDB**, **SNS** (with
    SNS→SQS fanout), **KMS**, **Secrets Manager**, **STS**, **CloudWatch
    Logs**, **SSM Parameter Store**, **EventBridge** (with EventBridge→SQS
    fanout), **Lambda** (metadata + echo-Invoke), **Kinesis**, **Step
    Functions**, **IAM**, **ECR**, **ELBv2**, **Route 53**, **API Gateway
    v1**, **ACM**, **Cognito**, **CloudWatch** (Query + RPC v2 CBOR),
    **CloudFormation**, **EC2** — all verified end-to-end with the AWS SDK
    for Rust.
  - Remaining 53: routed stubs that return a structured `UnsupportedOperation`
    (501) for every action. Each one is its own module under `src/services/`
    so coverage grows file-by-file.

## Quick start

```sh
cargo run                          # listens on 0.0.0.0:4566
KUROKO_DATA_DIR=./data cargo run   # persist state across restarts
```

Configure your AWS SDK to point at `http://localhost:4566`:

```rust
let cfg = aws_config::defaults(BehaviorVersion::latest())
    .endpoint_url("http://localhost:4566")
    .region("us-east-1")
    .credentials_provider(Credentials::new("test", "test", None, None, "kuroko"))
    .load().await;
let s3 = aws_sdk_s3::Client::from_conf(
    aws_sdk_s3::config::Builder::from(&cfg).force_path_style(true).build()
);
```

## E2E test coverage (AWS-spec aligned)

Every operation listed below is verified through the AWS SDK for Rust, with
assertions taken from the AWS official API reference docs (links are inlined
at the top of each test file).

| Service   | Operations covered |
|-----------|-------------------|
| S3        | CreateBucket, ListBuckets, HeadBucket (200/404), DeleteBucket (204, BucketNotEmpty), PutObject (ETag = MD5), GetObject (body + headers), HeadObject, DeleteObject, ListObjectsV2 (with prefix), x-amz-meta-* roundtrip |
| SQS       | CreateQueue, GetQueueUrl (incl. NonExistentQueue), ListQueues (prefix), SendMessage (MessageId + MD5), ReceiveMessage (max-clamp 1..=10), DeleteMessage (in-flight cleanup), SendMessageBatch, PurgeQueue, DeleteQueue, GetQueueAttributes (QueueArn shape) |
| DynamoDB  | CreateTable (TableArn shape, ResourceInUseException), ListTables (sorted), DescribeTable, PutItem, GetItem (missing → empty), DeleteItem, Scan (Count/ScannedCount), Query (KeyConditionExpression `=`, `<`, `<=`, `>`, `>=`, `BETWEEN`, `begins_with`, ExpressionAttributeNames, ScanIndexForward, Limit), BatchWriteItem, DeleteTable. Decimal comparison preserves full 38-digit `N` precision. |
| SNS       | CreateTopic (ARN shape), ListTopics, DeleteTopic (cascades to subscriptions), Publish (MessageId), Subscribe, Unsubscribe, ListSubscriptions, ListSubscriptionsByTopic, GetTopicAttributes, SetTopicAttributes, **SNS→SQS fanout** (publishing delivers an SNS envelope into the subscribed queue) |
| KMS       | CreateKey (Arn, KeyState), ListKeys, DescribeKey, CreateAlias, ListAliases, Encrypt, Decrypt, GenerateDataKey, EnableKey/DisableKey, ScheduleKeyDeletion, CancelKeyDeletion. Alias-based key references work end-to-end. Cipher is XOR (deterministic, **not secure** — emulator only). |
| SecretsMgr| CreateSecret (ResourceExistsException), GetSecretValue (AWSCURRENT/AWSPREVIOUS staging), PutSecretValue (current→previous rotation), UpdateSecret, DescribeSecret (VersionIdsToStages), ListSecrets, DeleteSecret (incl. ForceDeleteWithoutRecovery), accepts both names and full ARNs |
| STS       | GetCallerIdentity (Account/Arn), AssumeRole (Credentials), GetSessionToken, AssumeRoleWithWebIdentity, DecodeAuthorizationMessage |
| CWL Logs  | CreateLogGroup (ResourceAlreadyExistsException), DescribeLogGroups (prefix), CreateLogStream, PutLogEvents, GetLogEvents, FilterLogEvents (substring), DescribeLogStreams |
| SSM       | PutParameter (ParameterAlreadyExists / Overwrite-version-bump), GetParameter, GetParameters (separates found vs invalid), GetParametersByPath (shallow + recursive), DeleteParameter |
| EventBridge | CreateEventBus / DeleteEventBus (default protected), PutRule (EventPattern), Enable/DisableRule, PutTargets / RemoveTargets, ListTargetsByRule, **PutEvents → SQS fanout** (only matching + enabled rules deliver) |
| Lambda    | CreateFunction (ResourceConflictException), GetFunction (Configuration + Code), ListFunctions, UpdateFunctionConfiguration, DeleteFunction, **Invoke (echoes payload — kuroko does not run code)** |
| Kinesis   | CreateStream (ResourceInUseException), DescribeStream (single shard), ListStreams, PutRecord, PutRecords (batch), GetShardIterator (TRIM_HORIZON / LATEST / AT/AFTER_SEQUENCE_NUMBER), GetRecords (walks the log), DeleteStream |
| StepFns   | CreateStateMachine (InvalidDefinition for non-JSON), ListStateMachines, DescribeStateMachine, UpdateStateMachine, StartExecution (**immediately SUCCEEDED with input echoed as output**), DescribeExecution, ListExecutions (status filter), GetExecutionHistory (synthetic Started+Succeeded pair) |
| IAM       | CreateUser (EntityAlreadyExists), GetUser, ListUsers, DeleteUser (cascades access keys), CreateRole (AssumeRolePolicyDocument), CreatePolicy, AttachRolePolicy / ListAttachedRolePolicies, CreateAccessKey / ListAccessKeys |
| ECR       | CreateRepository (RepositoryAlreadyExistsException), DescribeRepositories, DeleteRepository (RepositoryNotEmptyException without force), PutImage (tag repointing), ListImages, BatchGetImage (by tag or digest), GetAuthorizationToken |
| ELBv2     | CreateLoadBalancer (DuplicateLoadBalancerNameException), CreateTargetGroup, RegisterTargets / DeregisterTargets, DescribeTargetHealth (all healthy), CreateListener (with default forward action), DeleteLoadBalancer cascades listeners |
| Route 53  | CreateHostedZone (returns `/hostedzone/<id>`), ListHostedZones, ChangeResourceRecordSets (CREATE / UPSERT replaces TTL+value / DELETE), ListResourceRecordSets, DeleteHostedZone |
| APIGW v1  | CreateRestApi (auto-creates root resource), GetResources (returns AWS-spec `item` singular), CreateResource (path composition), PutMethod / GetMethod, PutIntegration (AWS_PROXY etc.), CreateDeployment, CreateStage / GetStage |
| ACM       | RequestCertificate (immediate ISSUED), DescribeCertificate, ListCertificates, DeleteCertificate, ImportCertificate (Blob base64 decode), GetCertificate |
| Cognito   | CreateUserPool, ListUserPools, CreateUserPoolClient (GenerateSecret), AdminCreateUser (UsernameExistsException), AdminGetUser (UserAttributes field), AdminDeleteUser, AdminSetUserPassword |
| CloudWatch| PutMetricData, GetMetricStatistics (Sum/Average/Min/Max/SampleCount), ListMetrics (namespace+metric filter), PutMetricAlarm, DescribeAlarms, DeleteAlarms. **Both Query and Smithy RPC v2 CBOR** (with CBOR tag(1) timestamps) — works against the modern SDK's CBOR protocol migration. |
| CloudForm | CreateStack (AlreadyExistsException, immediate CREATE_COMPLETE), UpdateStack → UPDATE_COMPLETE, DescribeStacks, ListStacks, DescribeStackEvents, ListStackResources, GetTemplate, DeleteStack |
| EC2       | DescribeRegions (9 regions), DescribeAvailabilityZones, CreateVpc / DescribeVpcs / DeleteVpc, CreateSubnet (InvalidVpcID.NotFound), DescribeSubnets, CreateSecurityGroup, AuthorizeSecurityGroupIngress (IpPermissions.N), RunInstances (immediate running, MinCount/MaxCount), TerminateInstances (terminated state transition), DescribeInstances (reservation set), CreateTags |
| Admin     | `/_kuroko/health`, `/_kuroko/info`, `/_kuroko/services` (≥76 entries), `/_kuroko/reset` (clears all state) |
| Persist   | S3/SQS/DynamoDB write → snapshot → restart → restore — verified for each service. Snapshot path-traversal hardened (`[a-z0-9_-]+` only). |

Total: 184 tests across 27 suites, all verified against the AWS official API
reference docs (URLs inlined at the top of each `tests/e2e_*.rs` file).

## Endpoints

| Path | Purpose |
|------|---------|
| `POST /` | AWS JSON 1.0/1.1 and AWS Query unified dispatcher (Content-Type routes) |
| `POST /service/{svc}/operation/{op}` | Smithy RPC v2 CBOR |
| `GET /` | S3 ListBuckets (path-style) |
| `/{bucket}` and `/{bucket}/{*key}` | S3 REST |
| `GET /_kuroko/info` | kuroko version metadata |
| `GET /_kuroko/health` | Liveness |
| `GET /_kuroko/services` | List of registered services |
| `POST /_kuroko/reset` | Drop in-memory state for every service (test isolation) |

## Environment

| Variable | Default | Notes |
|----------|---------|-------|
| `KUROKO_HOST` | `0.0.0.0` | Bind host |
| `KUROKO_PORT` | `4566` | Bind port |
| `KUROKO_DATA_DIR` | (unset) | Enables JSON snapshot persistence |
| `KUROKO_LOG` | `info` | `tracing-subscriber` filter |

## Architecture

```
src/
  config.rs         CLI/env loading
  persistence.rs    JSON snapshot store with atomic rename
  aws_error.rs      Wire-protocol error envelopes (JSON / Query XML / REST XML / CBOR)
  service.rs        Service / JsonProtocolService / QueryProtocolService / CborProtocolService traits
  registry.rs       Lookup by target prefix, by Action, by Smithy service name
  protocol/
    awsjson.rs      Dispatch by X-Amz-Target
    query.rs        Dispatch by form `Action=` (SDK service id disambiguates collisions)
    cbor.rs         Dispatch by URL path (Smithy RPC v2)
  server.rs         axum wiring, unified POST `/` dispatcher, graceful shutdown
  services/
    stub.rs         Generic JSON/Query/CBOR/bare stubs returning 501
    s3.rs           Real S3 implementation (REST)
    sqs.rs          Real SQS implementation (AWS JSON 1.0)
    dynamodb.rs     Real DynamoDB implementation (AWS JSON 1.0)
    <73 service modules>
```

## Tests

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

Three integration tests in `tests/integration.rs` drive kuroko through the real
AWS SDK for Rust (S3 bucket/object roundtrip, SQS send/receive, DynamoDB
put/get).

## Adding a real implementation

Each service module today either implements a protocol trait directly or calls
`stub::register_*`. To replace a stub:

1. Open `src/services/<name>.rs`.
2. Define a struct, `impl Service` for it, and `impl JsonProtocolService` /
   `QueryProtocolService` / `CborProtocolService` as appropriate.
3. Swap the `register()` call (or move the registration into `services.rs`
   alongside `s3::S3::new()`).
4. Add an integration test in `tests/integration.rs`.

## License

MIT.
