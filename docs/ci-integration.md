# CI/CD Integration Guide

Guide for integrating tpu-doc into continuous integration and deployment pipelines.

## Exit Codes

tpu-doc uses standard exit codes for CI/CD integration:

| Code | Meaning | CI Action |
|------|---------|-----------|
| 0 | All checks passed | Continue pipeline |
| 1 | One or more checks failed | Fail pipeline |
| 2 | Warnings only (no failures) | Continue or warn |
| 3 | Runtime error | Fail pipeline |

---

## Output Formats

### JUnit XML

JUnit XML format is widely supported by CI systems for test result reporting.

```bash
tpu-doc check --format junit > results.xml
```

Output structure:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="tpu-doc" tests="31" failures="0" warnings="2" skipped="1" time="12.345">
  <testsuite name="Hardware" tests="6" failures="0" time="2.100">
    <testcase name="HW-001: TPU Device Detection" classname="Hardware" time="0.045">
    </testcase>
    <testcase name="HW-002: HBM Memory Availability" classname="Hardware" time="0.032">
    </testcase>
    <!-- ... -->
  </testsuite>
  <testsuite name="Stack" tests="7" failures="0" time="1.200">
    <!-- ... -->
  </testsuite>
  <!-- ... -->
</testsuites>
```

### JSON

JSON format for custom processing and storage.

```bash
tpu-doc check --format json > results.json
```

Output structure:
```json
{
  "timestamp": 1733580000,
  "hostname": "tpu-vm-001",
  "tpu_type": "v5e",
  "checks": [
    {
      "id": "HW-001",
      "name": "TPU Device Detection",
      "category": "Hardware",
      "result": {
        "status": "pass",
        "message": "8 chips detected",
        "duration_ms": 45
      }
    }
  ],
  "summary": {
    "passed": 28,
    "warned": 2,
    "failed": 0,
    "skipped": 1,
    "total": 31,
    "total_duration_ms": 12345
  }
}
```

---

## GitHub Actions

### Basic Workflow

```yaml
# .github/workflows/tpu-doc.yml
name: TPU Doc Validation

on:
  workflow_dispatch:
  push:
    branches: [main]

jobs:
  validate-tpu:
    runs-on: self-hosted  # Must run on TPU VM
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Download tpu-doc
        run: |
          curl -L -o tpu-doc https://github.com/clay-good/tpu-doc/releases/latest/download/tpu-doc-linux-x86_64
          chmod +x tpu-doc

      - name: Run TPU Doc Checks
        run: ./tpu-doc check --format junit > results.xml

      - name: Upload Results
        uses: actions/upload-artifact@v4
        if: always()
        with:
          name: tpu-doc-results
          path: results.xml

      - name: Publish Test Results
        uses: EnricoMi/publish-unit-test-result-action@v2
        if: always()
        with:
          files: results.xml
```

### Pre-Deployment Gate

```yaml
# .github/workflows/deploy.yml
name: Deploy to TPU

on:
  push:
    branches: [main]

jobs:
  tpu-doc-check:
    runs-on: self-hosted  # TPU VM runner
    outputs:
      status: ${{ steps.tpu_doc.outcome }}
    steps:
      - name: Run TPU Doc
        id: tpu_doc
        run: |
          ./tpu-doc check --fail-fast --quiet
          echo "status=success" >> $GITHUB_OUTPUT

  deploy:
    needs: tpu-doc-check
    if: needs.tpu-doc-check.outputs.status == 'success'
    runs-on: self-hosted
    steps:
      - name: Deploy Application
        run: |
          # Your deployment commands here
          echo "Deploying to TPU..."
```

### Baseline Comparison

```yaml
# .github/workflows/baseline-check.yml
name: Baseline Comparison

on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours

jobs:
  compare:
    runs-on: self-hosted
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Download Baseline
        run: |
          aws s3 cp s3://my-bucket/tpu-baselines/baseline.json baseline.json || true

      - name: Run TPU Doc with Baseline
        run: |
          ./tpu-doc check --format json --baseline baseline.json > current.json
        continue-on-error: true

      - name: Check for Regressions
        run: |
          if [ $? -eq 1 ]; then
            echo "::error::TPU Doc check failed - possible regression"
            exit 1
          fi

      - name: Update Baseline (on success)
        if: success()
        run: |
          aws s3 cp current.json s3://my-bucket/tpu-baselines/baseline.json
```

---

## GitLab CI

### Basic Pipeline

```yaml
# .gitlab-ci.yml
stages:
  - validate
  - deploy

tpu-doc-check:
  stage: validate
  tags:
    - tpu  # Runner on TPU VM
  script:
    - ./tpu-doc check --format junit > results.xml
  artifacts:
    when: always
    reports:
      junit: results.xml
    paths:
      - results.xml
    expire_in: 30 days

deploy:
  stage: deploy
  tags:
    - tpu
  needs:
    - tpu-doc-check
  script:
    - echo "Deploying application..."
  only:
    - main
```

### With Baseline Comparison

```yaml
# .gitlab-ci.yml
variables:
  BASELINE_PATH: /var/lib/tpu-doc/baseline.json

tpu-doc-check:
  stage: validate
  tags:
    - tpu
  script:
    - |
      if [ -f "$BASELINE_PATH" ]; then
        ./tpu-doc check --format json --baseline "$BASELINE_PATH" > results.json
      else
        ./tpu-doc check --format json > results.json
      fi
    - ./tpu-doc check --format junit > results.xml
  after_script:
    - |
      if [ "$CI_JOB_STATUS" == "success" ]; then
        cp results.json "$BASELINE_PATH"
      fi
  artifacts:
    when: always
    reports:
      junit: results.xml
    paths:
      - results.json
      - results.xml
```

---

## Jenkins

### Declarative Pipeline

```groovy
// Jenkinsfile
pipeline {
    agent {
        label 'tpu-vm'  // Jenkins agent on TPU VM
    }

    stages {
        stage('TPU Doc') {
            steps {
                sh './tpu-doc check --format junit > results.xml'
            }
            post {
                always {
                    junit 'results.xml'
                }
            }
        }

        stage('Deploy') {
            when {
                expression { currentBuild.resultIsBetterOrEqualTo('SUCCESS') }
            }
            steps {
                sh 'echo "Deploying to TPU..."'
            }
        }
    }

    post {
        failure {
            emailext (
                subject: "TPU Doc Failed: ${env.JOB_NAME}",
                body: "TPU Doc validation failed. Check ${env.BUILD_URL} for details.",
                to: 'team@example.com'
            )
        }
    }
}
```

### Scripted Pipeline

```groovy
// Jenkinsfile
node('tpu-vm') {
    stage('Checkout') {
        checkout scm
    }

    stage('TPU Doc') {
        def exitCode = sh(
            script: './tpu-doc check --format junit > results.xml',
            returnStatus: true
        )

        junit 'results.xml'

        if (exitCode == 1) {
            error('TPU Doc checks failed')
        } else if (exitCode == 2) {
            unstable('TPU Doc checks have warnings')
        } else if (exitCode == 3) {
            error('TPU Doc runtime error')
        }
    }

    stage('Deploy') {
        sh 'echo "Deploying..."'
    }
}
```

---

## Kubernetes Integration

### Init Container

Run tpu-doc as an init container before your main workload:

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tpu-workload
spec:
  replicas: 1
  selector:
    matchLabels:
      app: tpu-workload
  template:
    metadata:
      labels:
        app: tpu-workload
    spec:
      initContainers:
        - name: tpu-doc
          image: your-registry/tpu-doc:latest
          command: ["/tpu-doc"]
          args: ["check", "--fail-fast", "--quiet"]
          resources:
            limits:
              google.com/tpu: 8
      containers:
        - name: main
          image: your-registry/your-app:latest
          resources:
            limits:
              google.com/tpu: 8
```

### Pre-Deployment Job

```yaml
# tpu-doc-job.yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: tpu-doc
  annotations:
    argocd.argoproj.io/hook: PreSync
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: tpu-doc
          image: your-registry/tpu-doc:latest
          command: ["/tpu-doc"]
          args: ["check", "--format", "json"]
          resources:
            limits:
              google.com/tpu: 8
      nodeSelector:
        cloud.google.com/gke-tpu-topology: 2x4
        cloud.google.com/gke-tpu-accelerator: tpu-v5-lite-podslice
  backoffLimit: 0
```

### GKE TPU Node Pool Validation

```yaml
# daemonset-tpu-doc.yaml
apiVersion: apps/v1
kind: DaemonSet
metadata:
  name: tpu-doc
  namespace: kube-system
spec:
  selector:
    matchLabels:
      name: tpu-doc
  template:
    metadata:
      labels:
        name: tpu-doc
    spec:
      tolerations:
        - key: google.com/tpu
          operator: Exists
          effect: NoSchedule
      containers:
        - name: tpu-doc
          image: your-registry/tpu-doc:latest
          command: ["/bin/sh", "-c"]
          args:
            - |
              while true; do
                /tpu-doc check --format json > /var/log/tpu-doc/results.json
                sleep 3600  # Run every hour
              done
          volumeMounts:
            - name: results
              mountPath: /var/log/tpu-doc
          resources:
            limits:
              google.com/tpu: 8
      volumes:
        - name: results
          hostPath:
            path: /var/log/tpu-doc
      nodeSelector:
        cloud.google.com/gke-tpu: "true"
```

---

## Terraform Integration

### Provisioning with Validation

```hcl
# main.tf
resource "google_tpu_vm" "training" {
  name               = "training-tpu"
  zone               = "us-central1-b"
  accelerator_type   = "v5litepod-8"
  runtime_version    = "tpu-ubuntu2204-base"

  provisioner "remote-exec" {
    inline = [
      "curl -L -o /usr/local/bin/tpu-doc https://github.com/clay-good/tpu-doc/releases/latest/download/tpu-doc-linux-x86_64",
      "chmod +x /usr/local/bin/tpu-doc",
      "/usr/local/bin/tpu-doc check --fail-fast"
    ]

    connection {
      type        = "ssh"
      user        = "ubuntu"
      private_key = file("~/.ssh/id_rsa")
      host        = self.network_endpoints[0].ip_address
    }
  }
}
```

---

## Generic CI Integration

### Shell Script Wrapper

```bash
#!/bin/bash
# run-tpu-doc.sh
# Generic wrapper for CI systems

set -e

TPU_DOC_BIN="${TPU_DOC_BIN:-./tpu-doc}"
OUTPUT_DIR="${OUTPUT_DIR:-.}"
BASELINE="${BASELINE:-}"

# Run tpu-doc with all output formats
echo "Running tpu-doc validation..."

# Generate all output formats
$TPU_DOC_BIN check --format text > "$OUTPUT_DIR/results.txt" 2>&1 || true
$TPU_DOC_BIN check --format json > "$OUTPUT_DIR/results.json"
$TPU_DOC_BIN check --format junit > "$OUTPUT_DIR/results.xml"

# Get exit code from a clean run
$TPU_DOC_BIN check --quiet
EXIT_CODE=$?

# Report status
case $EXIT_CODE in
  0)
    echo "SUCCESS: All tpu-doc checks passed"
    ;;
  1)
    echo "FAILURE: One or more checks failed"
    cat "$OUTPUT_DIR/results.txt"
    ;;
  2)
    echo "WARNING: Checks passed with warnings"
    ;;
  3)
    echo "ERROR: Runtime error occurred"
    ;;
esac

# Compare to baseline if provided
if [ -n "$BASELINE" ] && [ -f "$BASELINE" ]; then
  echo "Comparing to baseline: $BASELINE"
  $TPU_DOC_BIN check --baseline "$BASELINE" --format json > "$OUTPUT_DIR/comparison.json"
fi

exit $EXIT_CODE
```

### Makefile Target

```makefile
# Makefile
.PHONY: tpu-doc tpu-doc-ci

TPU_DOC := ./tpu-doc

tpu-doc-check:
	$(TPU_DOC) check

tpu-doc-ci:
	$(TPU_DOC) check --format junit > results.xml
	$(TPU_DOC) check --format json > results.json

tpu-doc-strict:
	$(TPU_DOC) check --fail-fast

tpu-doc-hardware:
	$(TPU_DOC) check --hardware --fail-fast
```

---

## Best Practices

### When to Run TPU Doc

1. **Before every deployment** - Catch issues before they affect production
2. **After TPU provisioning** - Validate new TPU VMs before use
3. **Periodically (hourly/daily)** - Detect hardware degradation
4. **After maintenance windows** - Verify system health post-maintenance

### Handling Results

| Exit Code | Recommended Action |
|-----------|-------------------|
| 0 (Pass) | Proceed with deployment |
| 1 (Fail) | Block deployment, investigate failures |
| 2 (Warn) | Proceed with caution, review warnings |
| 3 (Error) | Block deployment, fix runtime issue |

### Warning vs Failure Strategy

**Conservative (Production)**
```bash
# Treat warnings as failures
tpu-doc check
if [ $? -ne 0 ]; then
  echo "Blocking deployment due to warnings or failures"
  exit 1
fi
```

**Permissive (Development)**
```bash
# Only fail on actual failures
tpu-doc check
if [ $? -eq 1 ] || [ $? -eq 3 ]; then
  echo "Blocking deployment due to failures"
  exit 1
fi
```

### Baseline Management

1. **Generate baseline after known-good state**
   ```bash
   tpu-doc check --format json > baseline.json
   ```

2. **Store baselines in version control or artifact storage**
   ```bash
   git add baseline.json
   # or
   aws s3 cp baseline.json s3://bucket/baselines/
   ```

3. **Update baseline after intentional changes**
   ```bash
   # After TPU upgrade, software update, etc.
   tpu-doc check --format json > baseline.json
   ```

4. **Compare against baseline in CI**
   ```bash
   tpu-doc check --baseline baseline.json
   ```

### Parallel Execution

For faster CI runs on multi-chip TPUs:
```bash
tpu-doc check --parallel --timeout 60000
```

### Selective Checks

Skip non-essential checks in CI:
```bash
# Skip informational firewall check
tpu-doc check --skip SEC-007

# Run only critical hardware checks
tpu-doc check --hardware --fail-fast
```

---

## Troubleshooting CI Issues

### Common Problems

**Problem: TPU Doc hangs in CI**
```bash
# Add timeout to prevent hanging
timeout 120 ./tpu-doc check --timeout 60000
```

**Problem: Color codes in CI logs**
```bash
# Disable colors
./tpu-doc check --no-color
# or
export NO_COLOR=1
```

**Problem: Permission denied errors**
```bash
# Ensure binary is executable
chmod +x ./tpu-doc
# Check TPU device permissions
ls -la /dev/accel*
```

**Problem: Network checks fail in isolated environment**
```bash
# Skip network-dependent checks
./tpu-doc check --skip IO-001 --skip IO-003 --skip IO-005
```
