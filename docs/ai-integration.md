# AI Integration Guide

This guide covers the optional AI-powered features in tpu-doc.

## Overview

tpu-doc includes optional AI-powered log analysis that can help diagnose issues in TPU training logs. This feature:

- Is strictly opt-in (requires `--ai` flag)
- Requires you to provide your own API key (BYOK - Bring Your Own Key)
- Supports Anthropic Claude and Google Gemini
- Never affects pass/fail decisions of validation checks
- Is built separately with the `ai` feature flag

## Building with AI Support

The AI features are compiled separately to keep the core binary dependency-free.

```bash
# Build without AI features (default)
cargo build --release

# Build with AI features
cargo build --release --features ai
```

The AI-enabled binary includes TLS support via rustls for secure API communication.

## Supported Providers

### Anthropic Claude

**Models:**
- claude-sonnet-4-20250514 (default)
- claude-3-haiku-20240307
- claude-3-opus-20240229

**Environment Variable:** `ANTHROPIC_API_KEY`

**API Endpoint:** https://api.anthropic.com/v1/messages

### Google Gemini

**Models:**
- gemini-1.5-pro (default)
- gemini-1.5-flash
- gemini-1.0-pro

**Environment Variable:** `GOOGLE_API_KEY`

**API Endpoint:** https://generativelanguage.googleapis.com/v1beta/models

## Setup

### 1. Obtain an API Key

**Anthropic:**
1. Create an account at https://console.anthropic.com/
2. Navigate to API Keys
3. Create a new API key
4. Copy the key (shown only once)

**Google:**
1. Go to https://makersuite.google.com/app/apikey
2. Create a new API key
3. Copy the key

### 2. Set Environment Variable

```bash
# For Anthropic Claude
export ANTHROPIC_API_KEY="sk-ant-..."

# For Google Gemini
export GOOGLE_API_KEY="AIza..."
```

For persistent configuration, add to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.).

### 3. Verify Setup

```bash
# Test with a simple log file
echo "Test log content" > /tmp/test.log
tpu-doc analyze /tmp/test.log --ai
```

## Usage

### Basic Analysis

```bash
# Analyze a training log with default provider (Anthropic)
tpu-doc analyze training.log --ai

# Use Google Gemini instead
tpu-doc analyze training.log --ai --provider google
```

### Asking Specific Questions

```bash
# Ask about a specific issue
tpu-doc analyze error.log --ai --question "Why is my training hanging after 1000 steps?"

# Ask about performance
tpu-doc analyze profile.log --ai --question "What are the main performance bottlenecks?"
```

### Specifying a Model

```bash
# Use a specific Anthropic model
tpu-doc analyze training.log --ai --model claude-3-haiku-20240307

# Use a specific Google model
tpu-doc analyze training.log --ai --provider google --model gemini-1.5-flash
```

### Output Formats

```bash
# Default text output
tpu-doc analyze training.log --ai

# JSON output for programmatic use
tpu-doc analyze training.log --ai --format json
```

## How It Works

When you run the analyze command, tpu-doc:

1. **Gathers Environment Context**
   - Runs the `info` command internally to capture TPU type, software versions, etc.
   - This context helps the AI understand your specific environment

2. **Reads the Log File**
   - Reads the specified log file (up to 10MB limit)
   - Truncates if necessary while preserving important sections

3. **Constructs the Prompt**
   - Combines environment context with log content
   - Adds your specific question if provided
   - Uses a system prompt optimized for TPU diagnostics

4. **Sends to AI Provider**
   - Makes an HTTPS request to the configured provider
   - Includes appropriate headers and authentication
   - Handles retries with exponential backoff

5. **Returns Analysis**
   - Displays the AI's analysis and recommendations
   - Includes environment context in JSON output

## Privacy Considerations

### What Gets Sent to the AI Provider

- **Log file content** - The contents of the log file you specify
- **Environment information** - TPU type, software versions, configuration
- **Your question** - If you provide one with `--question`

### What Does NOT Get Sent

- System credentials or API keys (other than the AI provider's key)
- Files other than the specified log file
- Network traffic or other system data
- Previous conversation history (each call is independent)

### Data Handling

- All communication uses HTTPS (TLS encryption)
- No data is cached or stored by tpu-doc
- AI providers have their own data retention policies:
  - Anthropic: See https://www.anthropic.com/privacy
  - Google: See https://ai.google.dev/terms

### Recommendations

- Review log files before sending to ensure no sensitive data
- Use environment variables for API keys, not command-line arguments
- Consider using the `--question` flag to focus analysis on specific issues
- For sensitive environments, consider self-hosted AI alternatives

## Troubleshooting

### API Key Not Found

```
Error: API key not found. Set ANTHROPIC_API_KEY environment variable.
```

**Solution:** Ensure the appropriate environment variable is set:
```bash
export ANTHROPIC_API_KEY="your-key-here"
```

### --ai Flag Required

```
Error: The --ai flag is required for the analyze command.
```

**Solution:** Add the `--ai` flag:
```bash
tpu-doc analyze training.log --ai
```

### AI Feature Not Enabled

```
Error: AI features require building with --features ai
```

**Solution:** Rebuild with AI features:
```bash
cargo build --release --features ai
```

### API Rate Limit

```
Error: API rate limit exceeded. Please wait and retry.
```

**Solution:** Wait a few seconds and retry. Consider using a smaller log file or the `--question` flag to focus the analysis.

### Request Timeout

```
Error: Request timed out after 60 seconds.
```

**Solution:**
- Check your network connection
- Try a smaller log file
- Use a faster model (e.g., claude-3-haiku or gemini-1.5-flash)

### Invalid API Key

```
Error: Authentication failed. Check your API key.
```

**Solution:**
- Verify the API key is correct
- Ensure the key has not expired or been revoked
- Check you're using the right key for the selected provider

### Log File Too Large

```
Error: Log file exceeds maximum size of 10MB.
```

**Solution:**
- Extract the relevant portion of the log
- Use grep to filter for error messages:
  ```bash
  grep -i "error\|exception\|failed" large.log > filtered.log
  tpu-doc analyze filtered.log --ai
  ```

## Cost Considerations

AI API usage incurs costs based on the provider's pricing:

### Anthropic Claude Pricing (approximate)
- Input tokens: $3-15 per million tokens
- Output tokens: $15-75 per million tokens
- Varies by model (Haiku is cheapest, Opus most expensive)

### Google Gemini Pricing (approximate)
- Input tokens: $0.50-7 per million tokens
- Output tokens: $1.50-21 per million tokens
- Varies by model (Flash is cheapest)

### Tips to Minimize Costs

1. **Use focused questions** - The `--question` flag helps get targeted responses
2. **Filter logs before analysis** - Extract only relevant portions
3. **Choose appropriate models** - Use Haiku or Flash for routine analysis
4. **Cache results** - Save JSON output for later reference

## Example Prompts

### Diagnosing OOM Errors
```bash
tpu-doc analyze training.log --ai --question "What caused the out-of-memory error and how can I fix it?"
```

### Understanding Slow Training
```bash
tpu-doc analyze profile.log --ai --question "Why is training slower than expected? What are the bottlenecks?"
```

### Investigating Hangs
```bash
tpu-doc analyze training.log --ai --question "The training appears to hang after step 5000. What might be causing this?"
```

### Checking XLA Compilation
```bash
tpu-doc analyze xla.log --ai --question "Are there any XLA compilation issues or warnings I should address?"
```

### General Health Check
```bash
tpu-doc analyze training.log --ai --question "Summarize any warnings, errors, or issues in this log."
```

## Integration with Other Commands

The analyze command works well in combination with other tpu-doc commands:

```bash
# First, capture environment info
tpu-doc info --format json > env.json

# Run validation checks
tpu-doc check --format json > checks.json

# Then analyze logs with full context
tpu-doc analyze training.log --ai --question "Given my environment, why might I be seeing these errors?"
```

## Limitations

- Maximum log file size: 10MB
- Requires network access to AI provider endpoints
- Analysis quality depends on log verbosity and content
- AI responses may occasionally be inaccurate
- No conversation memory between calls
- Cannot access files other than the specified log
