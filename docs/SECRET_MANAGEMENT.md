# Secret Management Configuration

**Purpose**: Secure handling of DEEPSEEK_API_KEY and other sensitive credentials

## GitHub Secrets Setup

### Required Secrets

| Secret Name | Example Value | Usage | Rotation |
|---|---|---|---|
| `DEEPSEEK_API_KEY` | `sk-xxxxxxxx...` | Daily benchmark tests | Monthly |
| `GITHUB_TOKEN` | (Auto-provided) | GitHub API access | Per-session |

### Setup Instructions

#### For ai-lib-rust Repository:

1. Go to: `https://github.com/[owner]/ai-lib-rust/settings/secrets/actions`
2. Click "New repository secret"
3. Add `DEEPSEEK_API_KEY`
   - Name: `DEEPSEEK_API_KEY`
   - Value: Your actual API key from Deepseek
4. Click "Add secret"

#### For ai-lib-python Repository:

1. Go to: `https://github.com/[owner]/ai-lib-python/settings/secrets/actions`
2. Add same `DEEPSEEK_API_KEY`
3. Click "Add secret"

#### For ai-lib-ts Repository:

1. Go to: `https://github.com/[owner]/ai-lib-ts/settings/secrets/actions`
2. Add same `DEEPSEEK_API_KEY`
3. Click "Add secret"

---

## Environment Variable Best Practices

### ✅ DO:
- Store all API keys in GitHub Secrets only
- Rotate keys monthly
- Use different keys for dev/staging/prod if possible
- Log API calls (without secrets) for audit trail
- Review secret access logs regularly

### ❌ DON'T:
- Commit `.env` files with real secrets
- Print secrets in logs or error messages
- Hardcode API keys in source code
- Share secrets across multiple projects unnecessarily
- Store backup keys in plaintext

---

## Local Development Setup

### Create `.env` file (NOT committed):

```bash
cat > .env << 'EOF'
DEEPSEEK_API_KEY=your_actual_key_here
BENCHMARK_DURATION=30
CONCURRENT_CONNECTIONS=5
EOF

# Add to .gitignore
echo ".env" >> .gitignore
echo ".env.local" >> .gitignore
```

### Load environment variables:

**PowerShell**:
```powershell
Get-Content .env | ForEach-Object {
    $parts = $_ -split '='
    [Environment]::SetEnvironmentVariable($parts[0], $parts[1], "Process")
}
```

**Bash**:
```bash
set -o allexport
source .env
set +o allexport
```

---

## Secret Rotation Schedule

### Monthly Rotation:
1. Generate new API key in Deepseek console
2. Update GitHub secret with new key
3. Verify all workflows pass with new key
4. Document in security log: "Key rotated: 2026-02-25"
5. Revoke old key after 7-day transition period

### Emergency Rotation:
- If key is compromised, rotate immediately
- Review access logs for unauthorized API calls
- Notify team in security channel
- File incident report

---

## Audit Trail

### Secret Access Logging

All secret access in CI/CD is automatically logged by GitHub:
- Location: Repository → Settings → Security & analysis → Secret scanning
- Review: Who accessed secrets, when, and from which workflow
- Retention: 90 days (GitHub default)

### Recommended Monitoring:
- Set up alerts for failed authentication attempts
- Monitor API usage rates (unusual spikes)
- Review CI/CD logs for unexpected secret usage

---

## Verification Steps

**1. Verify GitHub Secret is Set Correctly:**

```yaml
- name: Verify secret is available
  run: |
    if [ -z "${{ secrets.DEEPSEEK_API_KEY }}" ]; then
      echo "ERROR: DEEPSEEK_API_KEY secret not set!"
      exit 1
    else
      echo "✓ DEEPSEEK_API_KEY is configured"
    fi
```

**2. Test API Connection in Workflow:**

```yaml
- name: Test API connectivity
  env:
    DEEPSEEK_API_KEY: ${{ secrets.DEEPSEEK_API_KEY }}
  run: |
    curl -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
      https://api.deepseek.com/v1/chat/completions \
      -X POST \
      -H "Content-Type: application/json" \
      -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"test"}]}' \
      | head -c 100  # Don't print full response
```

**3. Verify No Secrets in Logs:**

```yaml
- name: Verify no secrets in logs
  if: always()
  run: |
    if grep -r "${{ secrets.DEEPSEEK_API_KEY }}" *.log; then
      echo "ERROR: Secret found in logs!"
      exit 1
    else
      echo "✓ No secrets exposed in logs"
    fi
```

---

## Branching & Secret Distribution

### Main Branch Only:
- Secrets are only available on `main` branch by default
- Forks do NOT inherit parent repository secrets
- Pull requests from forks cannot access secrets

### Feature Branches:
- Create feature branch from `main`
- Secrets auto-available for your own branches
- Ensure branch protection rules prevent unauthorized merges

---

## Documentation & Handoff

**Key Contacts**:
- Secrets administrator: [DevOps Team]
- Emergency contact: [On-call number]

**Documentation Location**:
- This file: `d:\rustapp\ai-lib-rust\docs\SECRET_MANAGEMENT.md`
- GitHub documentation: https://docs.github.com/en/actions/security-guides/encrypted-secrets

**Last Updated**: 2026-02-25
**Next Review**: 2026-03-25 (Monthly)
