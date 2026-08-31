---
name: zai-mcp
description: >
  Z.ai GLM Coding Plan tools via curl — web search fallback.
  Use when: Tavily/Exa/MiniMax are exhausted and you need a tertiary search provider.
  For GitHub repos → use repomix instead (far superior).
metadata:
  author: hodei-flow
  version: "1.3"
---

# Z.ai Tools via Curl

Z.ai provides web search via direct HTTP (curl) as a reliable fallback.
**For GitHub repos — use `repomix`** (see `repomix_pack_remote_repository`). It's far superior to z.ai's limited repo analysis.

---

## Web Search via Curl

### Command

```bash
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "webSearchPrime",
      "arguments": {
        "input": "YOUR SEARCH QUERY"
      }
    }
  }'
```

**Required env var** (set once):
```bash
export ZAI_API_KEY="<your-zai-api-key>"
```

### Examples

**Search for Rust async best practices:**
```bash
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"Rust async traits best practices 2026"}}}'
```

**Search for Kubernetes operators:**
```bash
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"Kubernetes operator pattern best practices"}}}'
```

### Output format

```json
{
  "result": {
    "organic": [
      {
        "title": "Result Title",
        "link": "https://example.com/article",
        "snippet": "Brief description of the result.",
        "date": "2026-01-15"
      }
    ],
    "related_searches": [{ "query": "related term" }]
  }
}
```

---

---

## GitHub Repo Analysis — Use Repomix Instead

**For GitHub repos, use `repomix`** — it's a far superior tool for code analysis.

```bash
# Pack a GitHub repo for analysis
repomix_pack_remote_repository(remote: "https://github.com/owner/repo")
```

This gives you the **complete packed code** — all files, not just summaries.

**Use z.ai curl only when repomix is not available.**

---

## Smart Usage Patterns

### 1. Research pipeline: Tavily → Exa → z.ai curl fallback

When Tavily and Exa fail or need cross-validation:
```bash
# z.ai as tertiary fallback
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"YOUR QUERY"}}}'
```

### 2. GitHub exploration pipeline

```
# Step 1: Get repo structure to understand layout
curl -s -X POST "https://api.z.ai/api/mcp/zread/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_repo_structure","arguments":{"repo_name":"owner/repo"}}}'

# Step 2: Ask about specific functionality
curl -s -X POST "https://api.z.ai/api/mcp/zread/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_doc","arguments":{"repo_name":"owner/repo","query":"your question"}}}'

# Step 3: Read specific file for implementation details
curl -s -X POST "https://api.z.ai/api/mcp/zread/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_file","arguments":{"repo_name":"owner/repo","file_path":"src/main.rs"}}}'
```

### 3. Cross-validate search results

```
# Same query to multiple providers
tavily_tavily_search(query: "WebAssembly component model best practices")
# If different results needed:
exa_web_search_exa(query: "WebAssembly component model best practices")
# And:
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" ... # same query
```

---

## Quota Notes

- Z.ai coding plan (Lite/Pro/Max): 100 / 1,000 / 4,000 calls per month
- Counts toward: web search + GitHub tools combined
- Check quota at: https://z.ai/console (or equivalent)

---

## Usage Examples

### Example 1: Research Rust async runtime alternatives

You need to evaluate Tokio vs async-std vs smol for a new project.

```bash
# Pack the repo with repomix for full analysis
repomix_pack_remote_repository(remote: "https://github.com/tokio-rs/tokio")

# Or use z.ai curl for a quick overview
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"tokio async runtime Rust features modules 2026"}}}'
```

### Example 2: Deep-dive on a specific algorithm

You found a paper referenced in a crate and want to understand the implementation.

```bash
# 1. Search for the concept
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"CRDT conflict-free replicated data types explained"}}}'

# 2. Read the crate's implementation
curl -s -X POST "https://api.z.ai/api/mcp/zread/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_file","arguments":{"repo_name":"bottsah/-CRDTs","file_path":"src/lib.rs"}}}'

# 3. Get repo structure to find relevant files
curl -s -X POST "https://api.z.ai/api/mcp/zread/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_repo_structure","arguments":{"repo_name":"bottsah/CRDTs"}}}'
```

### Example 3: Fallback search when Tavily/Exa are exhausted

```bash
# All three providers for the same query
# Tavily (primary)
tavily_tavily_search(query: "WebAssembly component model 2026")

# If Tavily fails → Exa
exa_web_search_exa(query: "WebAssembly component model 2026")

# If Exa also fails → z.ai curl (tertiary)
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"WebAssembly component model 2026"}}}'
```

### Example 4: Understand a new dependency before adding it

```bash
# Use repomix — packs the full repo for deep analysis
repomix_pack_remote_repository(remote: "https://github.com/serde-rs/serde")

# Use z.ai curl only for quick web search about the crate
curl -s -X POST "https://api.z.ai/api/mcp/web_search_prime/mcp" \
  -H "Authorization: Bearer $ZAI_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webSearchPrime","arguments":{"input":"serde Rust serialization JSON binary performance 2026"}}}'
```

---

## Quick Reference Card

```
┌──────────────────────────────────────────────────────┐
│  Z.ai — Tools via Curl                               │
├──────────────────────┬─────────────────────────────┤
│  Task               │ Command                       │
├──────────────────────┼─────────────────────────────┤
│  Web search         │ curl → web_search_prime      │
│  GitHub repos       │ repomix (superior)            │
├──────────────────────┴─────────────────────────────┤
│  Env: ZAI_API_KEY="<your-zai-api-key>"            │
└──────────────────────────────────────────────────────┘
```
