# Check Reference

Complete reference documentation for all tpu-preflight validation checks.

## Summary Table

| ID | Name | Category | Description |
|----|------|----------|-------------|
| HW-001 | TPU Device Detection | Hardware | Verify expected number of TPU chips are present |
| HW-002 | HBM Memory Availability | Hardware | Check total HBM capacity and availability |
| HW-003 | TPU Thermal Status | Hardware | Check temperature of each TPU chip |
| HW-004 | TPU Error Counters | Hardware | Check for accumulated hardware errors |
| HW-005 | ICI Interconnect Status | Hardware | Verify inter-chip interconnect is functional |
| HW-006 | Driver Status | Hardware | Verify TPU driver kernel module is loaded |
| STK-001 | JAX Version | Stack | Detect and validate installed JAX version |
| STK-002 | libtpu Version | Stack | Detect and validate libtpu version |
| STK-003 | XLA Compiler Version | Stack | Detect XLA compiler version |
| STK-004 | Python Version | Stack | Check Python version compatibility |
| STK-005 | PJRT Plugin Status | Stack | Verify PJRT TPU plugin is available |
| STK-006 | Dependency Conflicts | Stack | Check for known conflicting package versions |
| STK-007 | Environment Variables | Stack | Verify required environment variables are set |
| PERF-001 | MXU Utilization Test | Performance | Run standardized matrix multiplication and measure MXU utilization |
| PERF-002 | HBM Bandwidth Test | Performance | Measure HBM memory bandwidth |
| PERF-003 | Chip-to-Chip Latency | Performance | Measure latency between TPU chips |
| PERF-004 | Compilation Latency | Performance | Measure XLA compilation time for standard graph |
| PERF-005 | Memory Pressure Test | Performance | Allocate and free HBM to verify no fragmentation issues |
| IO-001 | GCS Read Throughput | I/O | Measure read throughput from Google Cloud Storage |
| IO-002 | Local Disk Throughput | I/O | Measure sequential read/write to local SSD |
| IO-003 | GCS Connectivity | I/O | Verify connectivity to storage.googleapis.com |
| IO-004 | Checkpoint Directory Access | I/O | Verify checkpoint directory access and space |
| IO-005 | Network Latency to GCP Services | I/O | Measure latency to GCP services |
| IO-006 | DNS Resolution | I/O | Verify DNS resolution is working |
| SEC-001 | Service Account Permissions | Security | Identify service account and check for overly permissive roles |
| SEC-002 | Network Exposure | Security | Check for services listening on all interfaces |
| SEC-003 | Workload Identity Status | Security | Check if workload identity is configured |
| SEC-004 | Encryption Status | Security | Verify data encryption settings |
| SEC-005 | Instance Metadata Access | Security | Verify metadata server access configuration |
| SEC-006 | SSH Key Management | Security | Check for OS Login vs legacy SSH keys |
| SEC-007 | Firewall Rules | Security | Provide guidance on firewall configuration |

---

## Hardware Checks

### HW-001: TPU Device Detection

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Verifies that the expected number of TPU chips are present and detectable on the system.

**What It Validates:**
- TPU chips are visible to the system
- Chip count matches expected configuration
- TPU VM environment is properly configured

**Method:**
1. Check environment variables for TPU configuration
2. Query sysfs entries under /sys/class/accel/
3. Use libtpu API if available
4. Compare detected count against expected (from TPU_CHIPS_PER_HOST or TPU type default)

**Pass Criteria:**
- Detected chip count equals expected chip count
- Message: "{count} chips detected"

**Warning Criteria:**
- More chips detected than expected (unusual configuration)
- Message: "More TPU chips than expected: {found} found, {expected} expected"

**Fail Criteria:**
- No TPU chips detected
- Fewer chips than expected detected
- Message: "Fewer TPU chips than expected: {found} found, {expected} expected"

**Skip Conditions:**
- Not running on a TPU VM

**Troubleshooting:**
- Verify TPU VM was provisioned correctly
- Check TPU driver is loaded (see HW-006)
- Verify TPU_NAME environment variable is set
- Check GCP Console for TPU health status

---

### HW-002: HBM Memory Availability

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** HW-001 (TPU detection)

**Description:**
Checks that HBM (High Bandwidth Memory) is available and not significantly consumed.

**What It Validates:**
- Total HBM capacity matches expected for TPU type
- Available HBM is above minimum threshold
- No memory leaks from previous workloads

**Method:**
1. Query HBM info from libtpu or sysfs
2. Calculate availability percentage
3. Compare against thresholds

**Pass Criteria:**
- HBM availability >= 90% of total
- Message: "{available_gb}GB available ({percentage}%)"

**Warning Criteria:**
- HBM availability between 50-90%
- Message: "HBM availability below threshold: {percentage}%"

**Fail Criteria:**
- HBM availability < 50%
- Message: "HBM availability critically low: {percentage}%"

**Skip Conditions:**
- Not running on a TPU VM
- HBM info unavailable

**Troubleshooting:**
- Check for zombie processes consuming HBM
- Restart TPU runtime if memory stuck
- Verify no other workloads are running

---

### HW-003: TPU Thermal Status

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** HW-001 (TPU detection)

**Description:**
Monitors temperature of each TPU chip to detect thermal throttling or overheating.

**What It Validates:**
- All chips are within safe operating temperature
- No thermal throttling is occurring
- Cooling system is functioning properly

**Method:**
1. Read temperature sensors from sysfs or libtpu
2. Find maximum temperature across all chips
3. Compare against warning (75C) and critical (85C) thresholds

**Pass Criteria:**
- All chips below 75C
- Message: "Max temperature: {temp}C"

**Warning Criteria:**
- Any chip between 75C and 85C
- Message: "TPU temperature elevated: {temp}C"

**Fail Criteria:**
- Any chip at or above 85C
- Message: "TPU temperature critical: {temp}C"

**Skip Conditions:**
- Not running on a TPU VM
- Thermal info unavailable

**Troubleshooting:**
- Check datacenter cooling
- Reduce workload intensity temporarily
- Verify no blocked airflow
- Contact GCP support for persistent issues

---

### HW-004: TPU Error Counters

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** HW-001 (TPU detection)

**Description:**
Checks hardware error counters for accumulated correctable and uncorrectable errors.

**What It Validates:**
- No uncorrectable hardware errors
- Correctable error count is acceptable
- TPU hardware is healthy

**Method:**
1. Read error counters from sysfs or libtpu
2. Check for uncorrectable errors (critical)
3. Check for correctable errors (warning)

**Pass Criteria:**
- Both correctable and uncorrectable error counts are zero
- Message: "No hardware errors"

**Warning Criteria:**
- Non-zero correctable errors (handled but may indicate degradation)
- Message: "{count} correctable errors detected"

**Fail Criteria:**
- Any uncorrectable errors present
- Message: "{count} uncorrectable errors detected"

**Skip Conditions:**
- Not running on a TPU VM
- Error counters unavailable

**Troubleshooting:**
- Report uncorrectable errors to GCP support
- Consider reprovisioning TPU with persistent errors
- Monitor error counts over time for trends

---

### HW-005: ICI Interconnect Status

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** HW-001 (TPU detection)

**Description:**
Verifies the Inter-Chip Interconnect (ICI) network between TPU chips is functional.

**What It Validates:**
- ICI links are healthy
- No interconnect errors
- Bandwidth is within expected range

**Method:**
1. Query ICI status from sysfs or libtpu
2. Check link health status
3. Measure or report bandwidth

**Pass Criteria:**
- ICI healthy, all links up
- Message: "ICI healthy, bandwidth: {bandwidth} GB/s"

**Warning Criteria:**
- N/A (ICI issues are typically critical)

**Fail Criteria:**
- ICI interconnect errors detected
- Links down or degraded
- Message: "ICI interconnect errors detected"

**Skip Conditions:**
- Not running on a TPU VM
- Single-chip configuration (ICI not applicable)
- ICI status unavailable

**Troubleshooting:**
- Single-chip VMs do not have ICI - skip is normal
- For multi-chip, report ICI failures to GCP support
- May require TPU reprovisioning

---

### HW-006: Driver Status

**Category:** Hardware
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Verifies the TPU kernel driver module is loaded and reports version information.

**What It Validates:**
- TPU driver kernel module is loaded
- Driver version is compatible
- Driver is functioning

**Method:**
1. Check /proc/modules or lsmod for TPU driver
2. Query driver version from sysfs
3. Optionally validate version compatibility

**Pass Criteria:**
- Driver loaded and version detected
- Message: "Driver version: {version}"

**Warning Criteria:**
- Driver loaded but version unknown
- Message: "Driver loaded but version unknown"

**Fail Criteria:**
- TPU driver not loaded
- Message: "TPU driver not loaded"

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Run `modprobe tpu` to load driver
- Check dmesg for driver errors
- Verify kernel version compatibility
- Reinstall TPU software stack

---

## Stack Checks

### STK-001: JAX Version

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Detects and validates the installed JAX version for TPU compatibility.

**What It Validates:**
- JAX is installed
- JAX version meets minimum requirements (0.4.1+)
- JAX version is compatible with TPU type

**Method:**
1. Check JAX_VERSION environment variable
2. Look for version in site-packages
3. Parse and compare version string

**Pass Criteria:**
- JAX version >= 0.4.1
- Message: "JAX version {version}"

**Warning Criteria:**
- JAX installed but version unparseable
- Message: "JAX version {version} (unparseable)"

**Fail Criteria:**
- JAX version < 0.4.1
- Message: "JAX version {version} is too old"

**Skip Conditions:**
- JAX version not detectable

**Troubleshooting:**
- Install/upgrade JAX: `pip install -U jax jaxlib`
- Use TPU-specific JAX version for best compatibility
- Check JAX release notes for TPU support

---

### STK-002: libtpu Version

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Detects and validates the installed libtpu version.

**What It Validates:**
- libtpu is installed
- Version is stable (not dev/nightly unless intentional)
- Version is compatible with TPU type

**Method:**
1. Check LIBTPU_VERSION environment variable
2. Query libtpu.so version string
3. Check TPU_LIBRARY_PATH location

**Pass Criteria:**
- libtpu version detected (stable)
- Message: "libtpu version {version}"

**Warning Criteria:**
- Using development/nightly build
- Message: "libtpu version {version}" with detail "Using development/nightly build"

**Fail Criteria:**
- N/A (version incompatibility not currently checked)

**Skip Conditions:**
- libtpu version unavailable

**Troubleshooting:**
- Verify TPU_LIBRARY_PATH is set correctly
- Reinstall libtpu from GCP repositories
- Match libtpu version to JAX version

---

### STK-003: XLA Compiler Version

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Detects the XLA compiler version (informational only).

**What It Validates:**
- XLA version is detectable
- XLA is available for compilation

**Method:**
1. Check XLA_VERSION environment variable
2. Query from JAX/TensorFlow if available

**Pass Criteria:**
- XLA version detected
- Message: "XLA version {version}"

**Warning Criteria:**
- N/A

**Fail Criteria:**
- N/A (informational only)

**Skip Conditions:**
- XLA version not detectable (common, informational only)

**Troubleshooting:**
- XLA is typically bundled with JAX
- No action needed if JAX is working

---

### STK-004: Python Version

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Checks that Python version meets minimum requirements.

**What It Validates:**
- Python 3.9 or later is installed
- Python is accessible

**Method:**
1. Check PYTHON_VERSION environment variable
2. Execute `python3 --version`
3. Parse version string

**Pass Criteria:**
- Python >= 3.9.0
- Message: "Python version {version}"

**Warning Criteria:**
- Python installed but version unparseable
- Message: "Python version {version} (unparseable)"

**Fail Criteria:**
- Python < 3.9
- Message: "Python version {version} is too old"

**Skip Conditions:**
- Python version unavailable

**Troubleshooting:**
- Install Python 3.9+
- Use pyenv or conda for version management
- Verify python3 is in PATH

---

### STK-005: PJRT Plugin Status

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Verifies the PJRT (Portable JAX Runtime) TPU plugin is available.

**What It Validates:**
- TPU_LIBRARY_PATH points to valid libtpu.so
- PJRT plugin is loadable
- Plugin location is correct

**Method:**
1. Check TPU_LIBRARY_PATH environment variable
2. Verify path exists
3. Check standard locations as fallback

**Pass Criteria:**
- PJRT plugin found at valid path
- Message: "PJRT plugin found at {path}"

**Warning Criteria:**
- TPU_LIBRARY_PATH not set (may still work)
- Message: "TPU_LIBRARY_PATH not set"

**Fail Criteria:**
- TPU_LIBRARY_PATH points to non-existent location
- Message: "TPU_LIBRARY_PATH points to non-existent location"

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Set TPU_LIBRARY_PATH=/usr/local/lib/libtpu.so
- Verify libtpu.so exists at expected location
- Reinstall TPU software stack

---

### STK-006: Dependency Conflicts

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Checks for known conflicting package version combinations.

**What It Validates:**
- No known problematic package combinations
- Version compatibility between dependencies

**Method:**
1. Check installed package versions
2. Compare against known conflict database
3. Report any detected conflicts

**Pass Criteria:**
- No known dependency conflicts
- Message: "No known dependency conflicts"

**Warning Criteria:**
- Potential conflicts detected
- Message: "{count} potential conflict(s) detected"

**Fail Criteria:**
- N/A (conflicts are warnings)

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Check specific conflict details
- Update conflicting packages
- Use virtual environment for isolation

---

### STK-007: Environment Variables

**Category:** Stack
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Verifies required and recommended environment variables are set.

**What It Validates:**
- Required: TPU_NAME
- Recommended: TPU_WORKER_ID, PYTHONPATH

**Method:**
1. Check for required environment variables
2. Check for recommended environment variables
3. Report missing variables

**Pass Criteria:**
- All required and recommended variables set
- Message: "All environment variables set"

**Warning Criteria:**
- Required present but recommended missing
- Message: "Missing recommended variable(s): {vars}"

**Fail Criteria:**
- Missing required environment variables
- Message: "Missing required environment variable(s): {vars}"

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Set TPU_NAME to your TPU resource name
- Set TPU_WORKER_ID for multi-host configurations
- Verify PYTHONPATH includes necessary paths

---

## Performance Checks

### PERF-001: MXU Utilization Test

**Category:** Performance
**Estimated Duration:** 10-30 seconds
**Dependencies:** HW-001 (TPU detection), STK-001 (JAX version)

**Description:**
Runs a standardized matrix multiplication to measure MXU (Matrix Multiply Unit) utilization.

**What It Validates:**
- TPU compute units are functioning
- MXU achieves expected utilization
- No performance degradation

**Method:**
1. Execute pre-compiled benchmark or Python/JAX harness
2. Measure achieved FLOPS vs theoretical peak
3. Calculate utilization percentage

**Pass Criteria:**
- MXU utilization > 80%
- Message: "MXU utilization: {percentage}%"

**Warning Criteria:**
- MXU utilization 70-80%
- Message: "MXU utilization below optimal: {percentage}%"

**Fail Criteria:**
- MXU utilization < 70%
- Message: "MXU utilization too low: {percentage}%"

**Skip Conditions:**
- Not running on a TPU VM
- MXU benchmark harness not available
- Requires JAX/Python harness

**Troubleshooting:**
- Check for other workloads consuming TPU
- Verify thermal status (throttling)
- Check for HBM memory pressure
- Reprovision TPU if persistent

---

### PERF-002: HBM Bandwidth Test

**Category:** Performance
**Estimated Duration:** 10-30 seconds
**Dependencies:** HW-001 (TPU detection), HW-002 (HBM availability)

**Description:**
Measures HBM memory bandwidth against expected baseline for TPU type.

**What It Validates:**
- HBM bandwidth meets minimum threshold
- Memory subsystem is healthy
- No bandwidth degradation

**Method:**
1. Execute memory bandwidth benchmark
2. Compare against expected bandwidth by TPU type:
   - v4: ~1200 GB/s
   - v5e: ~800 GB/s
   - v5p: ~1600 GB/s
   - v6e: ~1800 GB/s
   - v7: ~2000 GB/s

**Pass Criteria:**
- Measured bandwidth > 85% of expected
- Message: "HBM bandwidth: {bandwidth} GB/s ({percentage}% of expected)"

**Warning Criteria:**
- Measured bandwidth 70-85% of expected
- Message: "HBM bandwidth below optimal: {bandwidth} GB/s"

**Fail Criteria:**
- Measured bandwidth < 70% of expected
- Message: "HBM bandwidth too low: {bandwidth} GB/s"

**Skip Conditions:**
- Not running on a TPU VM
- HBM bandwidth test harness not available

**Troubleshooting:**
- Check for memory fragmentation
- Verify HBM availability (HW-002)
- Restart TPU runtime
- Contact GCP support for persistent issues

---

### PERF-003: Chip-to-Chip Latency

**Category:** Performance
**Estimated Duration:** 5-15 seconds
**Dependencies:** HW-001 (TPU detection), HW-005 (ICI status)

**Description:**
Measures communication latency between TPU chips via ICI interconnect.

**What It Validates:**
- Inter-chip communication is fast
- No ICI congestion or errors
- Multi-chip coordination is working

**Method:**
1. Execute latency benchmark between chips
2. Measure round-trip time in microseconds
3. Compare against expected (<10us for adjacent chips)

**Pass Criteria:**
- Latency < 20 microseconds
- Message: "Chip-to-chip latency: {latency}us"

**Warning Criteria:**
- Latency > 20 microseconds
- Message: "Chip-to-chip latency elevated: {latency}us"

**Fail Criteria:**
- N/A (elevated latency is warning)

**Skip Conditions:**
- Not running on a TPU VM
- Single-chip configuration
- Latency test harness not available

**Troubleshooting:**
- Single-chip VMs do not have chip-to-chip communication
- Check ICI status (HW-005)
- Report elevated latency to GCP support

---

### PERF-004: Compilation Latency

**Category:** Performance
**Estimated Duration:** 30-90 seconds
**Dependencies:** STK-001 (JAX version), STK-003 (XLA version)

**Description:**
Measures XLA compilation time for a standard computation graph.

**What It Validates:**
- XLA compiler is functioning
- Compilation speed is reasonable
- No compilation bottlenecks

**Method:**
1. Compile a standardized XLA graph
2. Measure compilation time
3. Compare against threshold (60 seconds)

**Pass Criteria:**
- Compilation time < 60 seconds
- Message: "XLA compilation time: {time}s"

**Warning Criteria:**
- Compilation time > 60 seconds
- Message: "XLA compilation unusually slow: {time}s"

**Fail Criteria:**
- N/A (slow compilation is warning)

**Skip Conditions:**
- Not running on a TPU VM
- Compilation test harness not available

**Troubleshooting:**
- Check CPU and memory utilization during compilation
- Verify no other compilation jobs running
- Clear XLA cache and retry

---

### PERF-005: Memory Pressure Test

**Category:** Performance
**Estimated Duration:** 10-30 seconds
**Dependencies:** HW-002 (HBM availability)

**Description:**
Allocates and frees HBM memory to verify no fragmentation or allocation issues.

**What It Validates:**
- HBM allocation works correctly
- No memory fragmentation
- Memory can be freed properly

**Method:**
1. Allocate large HBM buffers
2. Free buffers
3. Verify allocation/deallocation succeeds

**Pass Criteria:**
- Memory allocation/deallocation successful
- Message: "Memory allocation/deallocation successful"

**Warning Criteria:**
- N/A

**Fail Criteria:**
- OOM or fragmentation issues detected
- Message: "Memory pressure test failed"

**Skip Conditions:**
- Not running on a TPU VM
- Memory pressure test harness not available

**Troubleshooting:**
- Check for zombie processes holding memory
- Restart TPU runtime
- Verify HBM availability (HW-002)

---

## I/O Checks

### IO-001: GCS Read Throughput

**Category:** I/O
**Estimated Duration:** 5-30 seconds
**Dependencies:** IO-003 (GCS connectivity)

**Description:**
Measures read throughput from Google Cloud Storage.

**What It Validates:**
- GCS access is fast
- Network to GCS is not bottlenecked
- Can efficiently load checkpoints/data

**Method:**
1. Verify gsutil is available
2. Download test file from configured GCS bucket
3. Measure and report throughput

**Pass Criteria:**
- Throughput > 5 GB/s
- Message: "GCS throughput: {throughput} GB/s"

**Warning Criteria:**
- Throughput 2-5 GB/s
- Message: "GCS throughput below optimal: {throughput} GB/s"

**Fail Criteria:**
- Throughput < 2 GB/s
- Message: "GCS throughput too low: {throughput} GB/s"

**Skip Conditions:**
- Not running on GCP
- gsutil not available
- Test bucket not configured

**Troubleshooting:**
- Use same region for TPU and GCS bucket
- Check network configuration
- Verify service account has storage.objectViewer role

---

### IO-002: Local Disk Throughput

**Category:** I/O
**Estimated Duration:** 5-15 seconds
**Dependencies:** None

**Description:**
Measures sequential read/write throughput to local disk.

**What It Validates:**
- Local SSD is performing well
- No disk bottlenecks
- Can efficiently write checkpoints locally

**Method:**
1. Write 100MB test file using dd
2. Parse throughput from dd output
3. Clean up test file

**Pass Criteria:**
- Throughput >= 0.5 GB/s
- Message: "Local disk throughput: {throughput} GB/s"

**Warning Criteria:**
- Throughput < 0.5 GB/s
- Message: "Local disk throughput low: {throughput} GB/s"

**Fail Criteria:**
- N/A (low throughput is warning)

**Skip Conditions:**
- dd command fails

**Troubleshooting:**
- Check disk space
- Verify no other I/O intensive operations
- Check for disk errors in dmesg

---

### IO-003: GCS Connectivity

**Category:** I/O
**Estimated Duration:** <5 seconds
**Dependencies:** IO-006 (DNS resolution)

**Description:**
Verifies TCP connectivity to storage.googleapis.com.

**What It Validates:**
- Network path to GCS is available
- Firewall allows HTTPS traffic
- Service account can authenticate

**Method:**
1. Attempt TCP connection to storage.googleapis.com:443
2. Measure connection latency
3. Report success/failure

**Pass Criteria:**
- Connection succeeds
- Message: "GCS connectivity OK, latency: {latency}ms"

**Warning Criteria:**
- N/A

**Fail Criteria:**
- Cannot connect to storage.googleapis.com
- Message: "Cannot connect to storage.googleapis.com"

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Check firewall rules allow egress to GCS
- Verify DNS resolution (IO-006)
- Check service account permissions

---

### IO-004: Checkpoint Directory Access

**Category:** I/O
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Verifies the checkpoint directory is accessible with sufficient space.

**What It Validates:**
- CHECKPOINT_DIR exists or can be created
- Write permission is available
- Sufficient disk space (>100GB recommended)

**Method:**
1. Read CHECKPOINT_DIR environment variable
2. Create directory if needed
3. Test write permission
4. Check available space

**Pass Criteria:**
- Directory accessible with >= 100GB space
- Message: "Checkpoint directory OK, {space} GB available"

**Warning Criteria:**
- Space < 100GB
- Message: "Checkpoint directory space low: {space} GB available"

**Fail Criteria:**
- Cannot create directory
- No write permission
- Message: "No write permission for checkpoint directory"

**Skip Conditions:**
- CHECKPOINT_DIR not set

**Troubleshooting:**
- Set CHECKPOINT_DIR environment variable
- Check directory permissions
- Free disk space if low

---

### IO-005: Network Latency to GCP Services

**Category:** I/O
**Estimated Duration:** <5 seconds
**Dependencies:** IO-006 (DNS resolution)

**Description:**
Measures network latency to critical GCP services.

**What It Validates:**
- Low latency to GCP services
- Network path is optimal
- No routing issues

**Method:**
1. Measure TCP connection latency to:
   - metadata.google.internal:80
   - storage.googleapis.com:443
   - compute.googleapis.com:443
2. Report maximum latency

**Pass Criteria:**
- All services reachable with max latency < 10ms
- Message: "Network latency OK, max {latency}ms"

**Warning Criteria:**
- Max latency > 10ms OR some services unreachable
- Message: "Network latency elevated: max {latency}ms"

**Fail Criteria:**
- N/A (latency issues are warnings)

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Verify VPC configuration
- Check for network congestion
- Ensure TPU and services in same region

---

### IO-006: DNS Resolution

**Category:** I/O
**Estimated Duration:** <5 seconds
**Dependencies:** None

**Description:**
Verifies DNS resolution is working for critical hostnames.

**What It Validates:**
- DNS server is reachable
- GCP hostnames resolve correctly
- No DNS configuration issues

**Method:**
1. Resolve standard GCP hostnames:
   - storage.googleapis.com
   - metadata.google.internal
   - compute.googleapis.com
2. Measure resolution time

**Pass Criteria:**
- All hostnames resolve successfully
- Message: "DNS resolution OK, max {time}ms"

**Warning Criteria:**
- N/A

**Fail Criteria:**
- Any hostname fails to resolve
- Message: "DNS resolution failed"

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Check /etc/resolv.conf
- Verify VPC DNS settings
- Test with `nslookup` or `dig`

---

## Security Checks

### SEC-001: Service Account Permissions

**Category:** Security
**Estimated Duration:** <5 seconds
**Dependencies:** None

**Description:**
Identifies the service account and checks for overly permissive access scopes.

**What It Validates:**
- Service account is identifiable
- Access scopes follow least-privilege
- No unnecessary broad permissions

**Method:**
1. Query service account from metadata server
2. Query access scopes
3. Check for overly permissive scopes (cloud-platform, compute, devstorage.full)

**Pass Criteria:**
- Service account identified with appropriate scopes
- Message: "Service account: {email}"

**Warning Criteria:**
- Broad scopes detected
- Message: "Service account {email} has broad scopes"

**Fail Criteria:**
- N/A (permission issues are warnings)

**Skip Conditions:**
- Not running on GCP
- Service account info unavailable

**Troubleshooting:**
- Use minimal required scopes
- Consider custom service account
- Review IAM roles in GCP Console

---

### SEC-002: Network Exposure

**Category:** Security
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Checks for services listening on all network interfaces (0.0.0.0).

**What It Validates:**
- Services are not unnecessarily exposed
- Sensitive ports are bound to localhost
- No unintended network exposure

**Method:**
1. Parse /proc/net/tcp and /proc/net/tcp6
2. Find sockets listening on 0.0.0.0 or ::
3. Flag concerning ports (22, 80, 443, 8080, 8888, etc.)

**Pass Criteria:**
- No concerning ports exposed OR no services on all interfaces
- Message: "No services exposed on all interfaces"

**Warning Criteria:**
- Common ports listening on all interfaces
- Message: "{count} potentially exposed port(s): {ports}"

**Fail Criteria:**
- N/A (exposure is warning)

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Bind services to 127.0.0.1 when possible
- Review firewall rules
- Use VPC firewall to restrict access

---

### SEC-003: Workload Identity Status

**Category:** Security
**Estimated Duration:** <5 seconds
**Dependencies:** None

**Description:**
Checks if workload identity is configured (recommended for GKE).

**What It Validates:**
- Using custom service account (not default)
- Workload identity configured if on GKE
- Following security best practices

**Method:**
1. Check for GKE cluster indicators
2. Check if using default Compute Engine service account
3. Report configuration status

**Pass Criteria:**
- Using custom service account OR workload identity
- Message: "Using custom service account: {email}"

**Warning Criteria:**
- Using default Compute Engine service account
- Message: "Using default Compute Engine service account"

**Fail Criteria:**
- N/A (configuration issues are warnings)

**Skip Conditions:**
- Not running on GCP
- Cannot determine configuration

**Troubleshooting:**
- Create custom service account with minimal permissions
- Configure workload identity for GKE
- Avoid using default service account

---

### SEC-004: Encryption Status

**Category:** Security
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Verifies data encryption settings (informational).

**What It Validates:**
- GCP default encryption is active
- CMEK usage if configured

**Method:**
1. Report GCP default encryption status
2. Check for CMEK indicators

**Pass Criteria:**
- GCP encryption at rest enabled (always true on GCP)
- Message: "GCP default encryption at rest enabled"

**Warning Criteria:**
- N/A

**Fail Criteria:**
- N/A

**Skip Conditions:**
- Not running on GCP

**Troubleshooting:**
- GCP always encrypts data at rest
- Configure CMEK for additional control
- Review encryption settings in GCP Console

---

### SEC-005: Instance Metadata Access

**Category:** Security
**Estimated Duration:** <5 seconds
**Dependencies:** None

**Description:**
Verifies metadata server access is appropriately configured.

**What It Validates:**
- Metadata server requires proper headers
- No unrestricted metadata access
- Metadata concealment if available

**Method:**
1. Attempt to access metadata server without headers
2. Check response code (403 = protected, 200 = open)
3. Report configuration status

**Pass Criteria:**
- Metadata requires proper headers (403 response)
- Message: "Metadata access requires proper headers"

**Warning Criteria:**
- Metadata accessible without protection
- Message: "Metadata server accessible without protection headers"

**Fail Criteria:**
- N/A (open access is warning)

**Skip Conditions:**
- Not running on GCP
- Cannot check metadata access

**Troubleshooting:**
- Enable metadata concealment
- Configure instance with restricted metadata
- Review GCP security best practices

---

### SEC-006: SSH Key Management

**Category:** Security
**Estimated Duration:** <5 seconds
**Dependencies:** None

**Description:**
Checks for OS Login vs legacy SSH key management.

**What It Validates:**
- OS Login enabled (recommended)
- Not using project-wide SSH keys
- Centralized key management

**Method:**
1. Check enable-oslogin instance attribute
2. Report OS Login status

**Pass Criteria:**
- OS Login enabled
- Message: "OS Login enabled"

**Warning Criteria:**
- OS Login not enabled
- Message: "OS Login not enabled"

**Fail Criteria:**
- N/A (configuration is warning)

**Skip Conditions:**
- Not running on GCP

**Troubleshooting:**
- Enable OS Login in instance metadata
- Use `gcloud compute instances add-metadata` to enable
- Review GCP OS Login documentation

---

### SEC-007: Firewall Rules

**Category:** Security
**Estimated Duration:** <1 second
**Dependencies:** None

**Description:**
Provides guidance on firewall configuration (cannot check from instance).

**What It Validates:**
- Informational only - firewall rules cannot be queried from instance

**Method:**
1. Return informational message about checking firewall rules

**Pass Criteria:**
- Always passes (informational)
- Message: "Firewall rules must be verified via GCP Console or gcloud"

**Warning Criteria:**
- N/A

**Fail Criteria:**
- N/A

**Skip Conditions:**
- None (always runs)

**Troubleshooting:**
- Review firewall rules in GCP Console
- Use `gcloud compute firewall-rules list`
- Ensure only required ports are open
