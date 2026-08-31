---
name: minimax-mcp
description: >
  MiniMax Coding Plan MCP tools — web search and AI image understanding.
  Use when: searching the web for current information, analyzing images (screenshots, diagrams, photos),
  or doing visual QA. Tools: web_search, understand_image.
metadata:
  author: hodei-flow
  version: "1.0"
---

# MiniMax MCP Tools

MiniMax Coding Plan provides two production-ready MCP tools via `minimax-coding-plan-mcp`:
`web_search` and `understand_image`. Both are already configured in your opencode.json.

## Tools

### `minimax_web_search` — Web Search

Performs a web search and returns organic results with titles, URLs, snippets, and related queries.

**When to use:**
- General web searches when Tavily/Exa are unavailable or exhausted
- Cross-validating results from other search providers
- Quick factual queries

**Parameters:**
| Param | Type | Description |
|-------|------|-------------|
| `query` | string | Search query (required) |

**Examples:**
```
minimax_web_search(query: "Rust async traits best practices 2026")
minimax_web_search(query: "latest Next.js 15 release notes")
```

**Output format:**
```json
{
  "organic": [
    {
      "title": "Result Title",
      "link": "https://example.com/article",
      "snippet": "Brief description of the result.",
      "date": "2026-01-15"
    }
  ],
  "related_searches": [{ "query": "related term" }],
  "base_resp": { "status_code": 0, "status_msg": "success" }
}
```

**Quota**: Check your MiniMax coding plan limits (Lite/Pro/Max have different quotas).

---

### `minimax_understand_image` — AI Image Analysis

Analyzes images with AI using MiniMax's vision model. Supports URL or local file input.

**When to use:**
- Analyzing screenshots of UI, terminals, or error messages
- Reading diagrams, architecture drawings, flowcharts
- Visual QA — comparing expected vs actual renders
- Extracting information from photos or scanned documents
- Understanding technical diagrams (UML, ER, architecture)

**Parameters:**
| Param | Type | Description |
|-------|------|-------------|
| `prompt` | string | What to ask about the image (required) |
| `image_url` | string | Public URL to the image (optional, mutually exclusive with image_path) |
| `image_path` | string | Local file path to the image (optional, mutually exclusive with image_url) |

**Examples:**
```
// Analyze a UI screenshot
minimax_understand_image(
  prompt: "What components are visible? Describe the layout and any obvious issues.",
  image_url: "https://example.com/ui-screenshot.png"
)

// Analyze an error terminal screenshot
minimax_understand_image(
  prompt: "What error is shown? Extract the error message, type, and any stack trace.",
  image_path: "/tmp/test-error.png"
)

// Analyze a technical architecture diagram
minimax_understand_image(
  prompt: "Describe the architecture shown. What are the main components and how do they communicate?",
  image_url: "https://example.com/architecture.png"
)

// Compare two UI screenshots (after getting both)
minimax_understand_image(
  prompt: "Compare this with the reference design. List all visual differences.",
  image_url: "https://example.com/actual-render.png"
)
```

**Output format:** Natural language description returned as a string.

**Supported formats:** JPEG, PNG, WebP (from URL or local path).

---

## Smart Usage Patterns

### 1. Image-first debugging
When diagnosing a bug, ask the agent to screenshot the failing UI/render.
Then use `understand_image` to analyze it:
```
The button is invisible on dark backgrounds.
Analyze: minimax_understand_image(prompt: "Describe the button appearance against the dark background. Is there sufficient contrast?", image_path: "/tmp/button-dark.png")
```

### 2. Visual regression catch
After a UI change, capture before/after screenshots and analyze:
```
minimax_understand_image(prompt: "List all visual differences between this and the reference: what changed in layout, colors, spacing, components?", image_url: "https://example.com/after.png")
```

### 3. Architecture diagram extraction
When exploring a new codebase, ask for architecture diagrams:
```
minimax_understand_image(prompt: "Extract and list all components, their responsibilities, and the arrows/relationships between them.", image_url: "https://example.com/diagram.png")
```

### 4. Error screenshot triage
For frontend errors, screenshot the console/network tab:
```
minimax_understand_image(prompt: "Extract: error type, message, stack trace frames, and which line/column the error originates from.", image_path: "/tmp/console-error.png")
```

---

## Integration with Search Strategy

MiniMax is the **fallback provider** in the multi-provider search strategy:

| Priority | Provider | When to use |
|----------|----------|-------------|
| 1 | Tavily | Curated, high-quality results for technical docs |
| 2 | Exa | Broad coverage, fresh content, news |
| 3 | **MiniMax** | When 1 and 2 fail or for image understanding |

```python
# Search fallback logic (pseudocode)
if tavily_result.is_rate_limited:
    result = exa_web_search_exa(query)
    if exa_result.is_rate_limited:
        result = minimax_web_search(query)  # last resort
else:
    result = tavily_result
```

---

## Quota Notes

- MiniMax coding plan has monthly quotas (Lite: limited, Pro: 1000 calls, Max: 4000 calls)
- Web search and image understanding share the same quota pool
- Monitor usage at: https://platform.minimax.io/user-center/payment/token-plan

---

## Usage Examples

### Example 1: Visual QA — check UI renders correctly

After a CSS change, verify the button looks correct in dark mode:

```bash
# Take a screenshot of the rendered button
# Then analyze it
minimax_understand_image(
  prompt: "Describe the button appearance. Is the text readable against the dark background? Rate contrast 1-10.",
  image_path: "/tmp/button-dark-mode.png"
)
```

### Example 2: Extract architecture from a diagram

Found a diagram in a paper or blog post:

```bash
minimax_understand_image(
  prompt: "List all components shown, their labels, and describe the arrows/relationships between them. Extract any numbered steps or flow sequences.",
  image_url: "https://example.com/architecture-diagram.png"
)
```

### Example 3: Error screenshot triage

Get a terminal screenshot showing an error:

```bash
minimax_understand_image(
  prompt: "Extract: error type, error message, stack trace frames (file:line:col for each), and which line the error originates from.",
  image_path: "/tmp/error-terminal.png"
)
```

### Example 4: Compare before/after UI change

```bash
# After making a UI change, capture both versions
minimax_understand_image(
  prompt: "Describe this UI. List all visible components and their positions.",
  image_url: "https://example.com/before.png"
)
minimax_understand_image(
  prompt: "Compare with reference. List ALL visual differences: layout shifts, color changes, missing/added components, spacing issues.",
  image_url: "https://example.com/after.png"
)
```

### Example 5: Cross-validate search results (fallback)

```bash
# When Tavily + Exa both fail or need diversity
minimax_web_search(query: "CRDT conflict-free replicated data types Rust implementation 2026")
```

---

## File Locations

- **MCP config**: `~/.config/opencode/opencode.json` (section `mcp.minimax`)
- **Server**: `uvx minimax-coding-plan-mcp -y` (local)
- **API key**: Stored in `MINIMAX_API_KEY` env var in the MCP config
