# argocd-pr-preview-demo

## Intent

This directory is for verifying a Kubernetes and Argo CD based preview-environment workflow per pull request.

## Working Rules

- Optimize for local reproducibility first: Kind before any cloud-specific setup.
- Keep manifests, scripts, and notes small and explicit so the lifecycle is easy to inspect.
- Treat this as an experiment, but preserve enough structure that the result can be turned into a blog post.
- When adding components, document both startup and cleanup paths.
