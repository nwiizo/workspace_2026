//! Service catalog. Every AWS service kuroko recognizes is registered here.
//!
//! Three protocol families:
//!   - REST    : own axum router (S3, S3 Control, S3 Tables, Lambda, etc.)
//!   - JSON    : dispatch via X-Amz-Target (SQS, DynamoDB, KMS, EventBridge, ...)
//!   - Query   : dispatch via Action= form params (EC2, RDS, IAM, SNS, ...)
//!   - CBOR    : Smithy RPC v2 (CloudWatch and friends)
//!
//! Services without full coverage are wired to `stub::generic` which returns a
//! 501 UnsupportedOperation envelope. As coverage grows, each stub is replaced
//! with a real implementation.

use std::sync::Arc;

use crate::registry::Registry;

pub mod stub;

// === Fully implemented services ===
pub mod dynamodb;
pub mod s3;
pub mod sqs;

// === Stub modules (one per service for clean per-service growth) ===
pub mod acm;
pub mod amplify;
pub mod apigateway;
pub mod appmesh;
pub mod appsync;
pub mod athena;
pub mod backup;
pub mod batch;
pub mod ce;
pub mod cloudcontrol;
pub mod cloudformation;
pub mod cloudfront;
pub mod cloudtrail;
pub mod cloudwatch;
pub mod cloudwatchlogs;
pub mod codeconnections;
pub mod codeguruprofiler;
pub mod codegurureviewer;
pub mod cognito;
pub mod comprehend;
pub mod configservice;
pub mod dataexchange;
pub mod dlm;
pub mod documentdb;
pub mod ds;
pub mod ebs;
pub mod ec2;
pub mod ecr;
pub mod ecs;
pub mod eks;
pub mod elasticache;
pub mod elasticbeanstalk;
pub mod elbv2;
pub mod emrserverless;
pub mod entityresolution;
pub mod eventbridge;
pub mod finspace;
pub mod firehose;
pub mod forecast;
pub mod gamelift;
pub mod glacier;
pub mod globalaccelerator;
pub mod glue;
pub mod iam;
pub mod kafka;
pub mod kinesis;
pub mod kms;
pub mod lambda;
pub mod location;
pub mod macie2;
pub mod memorydb;
pub mod mq;
pub mod neptune;
pub mod organizations;
pub mod pinpointsmsvoicev2;
pub mod pipes;
pub mod rds;
pub mod redshift;
pub mod rekognition;
pub mod resiliencehub;
pub mod route53;
pub mod route53resolver;
pub mod s3control;
pub mod s3tables;
pub mod sagemaker;
pub mod scheduler;
pub mod secretsmanager;
pub mod securitylake;
pub mod servicequotas;
pub mod ses;
pub mod sesv2;
pub mod sfn;
pub mod sns;
pub mod ssm;
pub mod sts;

pub fn register_all(registry: &Arc<Registry>) {
    // Fully implemented (own routers / dispatchers)
    registry.register(Arc::new(s3::S3::new()));
    registry.register_json(Arc::new(sqs::Sqs::new()));
    registry.register_json(Arc::new(dynamodb::DynamoDb::new()));

    // Stubs — each module declares its target_prefix / actions / smithy name so
    // SDKs reach the dispatcher and get a structured 501 instead of a 404.
    acm::register(registry);
    amplify::register(registry);
    apigateway::register(registry);
    appmesh::register(registry);
    appsync::register(registry);
    athena::register(registry);
    backup::register(registry);
    batch::register(registry);
    ce::register(registry);
    cloudcontrol::register(registry);
    cloudformation::register(registry);
    cloudfront::register(registry);
    cloudtrail::register(registry);
    cloudwatch::register(registry);
    cloudwatchlogs::register(registry);
    codeconnections::register(registry);
    codeguruprofiler::register(registry);
    codegurureviewer::register(registry);
    cognito::register(registry);
    comprehend::register(registry);
    configservice::register(registry);
    dataexchange::register(registry);
    dlm::register(registry);
    documentdb::register(registry);
    ds::register(registry);
    ebs::register(registry);
    ec2::register(registry);
    ecr::register(registry);
    ecs::register(registry);
    eks::register(registry);
    elasticache::register(registry);
    elasticbeanstalk::register(registry);
    elbv2::register(registry);
    emrserverless::register(registry);
    entityresolution::register(registry);
    eventbridge::register(registry);
    finspace::register(registry);
    firehose::register(registry);
    forecast::register(registry);
    gamelift::register(registry);
    glacier::register(registry);
    globalaccelerator::register(registry);
    glue::register(registry);
    iam::register(registry);
    kafka::register(registry);
    kinesis::register(registry);
    kms::register(registry);
    lambda::register(registry);
    location::register(registry);
    macie2::register(registry);
    memorydb::register(registry);
    mq::register(registry);
    neptune::register(registry);
    organizations::register(registry);
    pinpointsmsvoicev2::register(registry);
    pipes::register(registry);
    rds::register(registry);
    redshift::register(registry);
    rekognition::register(registry);
    resiliencehub::register(registry);
    route53::register(registry);
    route53resolver::register(registry);
    s3control::register(registry);
    s3tables::register(registry);
    sagemaker::register(registry);
    scheduler::register(registry);
    secretsmanager::register(registry);
    securitylake::register(registry);
    servicequotas::register(registry);
    ses::register(registry);
    sesv2::register(registry);
    sfn::register(registry);
    sns::register(registry);
    ssm::register(registry);
    sts::register(registry);
}
