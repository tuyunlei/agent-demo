# Agent Skills 调研

## 1. Agent Skills 开放标准（agentskills.io）

Anthropic 发起，已成为开放标准，被 30+ 产品采纳（Claude Code、Cursor、Codex、Gemini CLI、JetBrains Junie、OpenHands、Roo Code 等）。

### 核心定义

**Skill = 一个目录，最少包含一个 SKILL.md 文件。**

```
skill-name/
├── SKILL.md          # 必须：frontmatter + markdown 指令
├── scripts/          # 可选：可执行脚本
├── references/       # 可选：参考文档
└── assets/           # 可选：模板、图片、数据文件
```

### SKILL.md 格式

```yaml
---
name: pdf-processing          # 必须，1-64字符，小写+数字+连字符
description: Extract text...  # 必须，1-1024字符，描述功能+触发条件
license: Apache-2.0           # 可选
compatibility: Requires git   # 可选，1-500字符
metadata:                     # 可选，任意 key-value
  author: example-org
  version: "1.0"
allowed-tools: Bash(git:*) Read  # 可选，实验性
---

# PDF Processing
...step-by-step instructions...
```

### 约束规则

| 字段 | 约束 |
|------|------|
| name | 1-64字符，`[a-z0-9-]`，不能 `-` 开头结尾，不能 `--`，必须等于目录名 |
| description | 1-1024字符，必须非空，应包含触发关键词 |
| SKILL.md 整体 | 建议 <500 行、<5000 token |
| 单个文件大小 | OpenClaw 默认限制 256KB |

### 三层渐进披露（Progressive Disclosure）

标准的核心设计理念：

1. **发现层**（~100 token/skill）：启动时只加载 name + description → 注入 system prompt
2. **激活层**（<5000 token）：任务匹配时读取完整 SKILL.md body
3. **资源层**（按需）：执行时才加载 scripts/references/assets

### 两种集成方式

1. **Filesystem-based**：agent 有 shell 环境，通过 `cat /path/to/SKILL.md` 读取
2. **Tool-based**：agent 无 shell 环境，通过自定义工具触发 skill

### Prompt 注入格式（推荐 XML）

```xml
<available_skills>
  <skill>
    <name>pdf-processing</name>
    <description>Extract text and tables from PDF files...</description>
    <location>/path/to/skills/pdf-processing/SKILL.md</location>
  </skill>
</available_skills>
```

---

## 2. OpenClaw 的实现

### 架构总览

```
                      ┌─── workspace/skills/     （项目级，最高优先级）
                      ├─── .agents/skills/        （项目级 .agents 标准）
Skill Sources ────────┼─── ~/.agents/skills/      （用户级 .agents 标准）
                      ├─── ~/.openclaw/skills/    （managed，openclaw install）
                      ├─── 内置 skills             （bundled）
                      └─── config extraDirs       （额外目录）

          ↓ loadSkillEntries()

   SkillEntry[] ──→ filterSkillEntries() ──→ formatSkillsForPrompt()
                                                      ↓
                                            注入 system prompt
```

### 加载流程

**优先级**（后覆盖前）：extra < bundled < managed < agents-personal < agents-project < workspace

1. 扫描各来源目录，找 `*/SKILL.md` 文件
2. 解析 YAML frontmatter → name/description
3. 提取 OpenClaw 扩展元数据（always/requires/install 等）
4. 按 name 去重合并（同名后者覆盖）
5. 资格检查（OS、依赖二进制、环境变量）
6. 生成 `<available_skills>` XML 注入 system prompt

### OpenClaw 扩展字段

标准之外，OpenClaw 在 frontmatter 中支持：

| 字段 | 作用 |
|------|------|
| `always: true` | 每次对话都加载，不靠 agent 判断 |
| `user-invocable: false` | 不注册为 `/命令` |
| `disable-model-invocation: true` | 不出现在 prompt 中 |
| `requires.bins/env/config` | 资格检查 |
| `install` | 安装规格（brew/node/go/uv/download） |
| `command-dispatch: tool` | 用户命令直接转发到工具调用 |

### Prompt 保护限制

| 参数 | 默认值 |
|------|--------|
| maxCandidatesPerRoot | 300 |
| maxSkillsLoadedPerSource | 200 |
| maxSkillsInPrompt | 150 |
| maxSkillsPromptChars | 30,000 |
| maxSkillFileBytes | 256,000 |

### 关键设计点

**Agent 自己决定什么时候激活 skill。** 不是硬编码路由，是 LLM 根据 description 做语义匹配判断。

---

## 3. PicoClaw (Go) 的最小实现

- `SkillsLoader`：扫描 workspace/global/builtin 三目录
- `ListSkills()`：收集 name/description
- `LoadSkill(name)`：读取并去除 frontmatter
- `BuildSkillsSummary()`：生成 `<skills>` XML

无扩展字段、无限制保护、无命令系统。接近标准的最小实现。

---

## 4. Skill 与 Tool 的关系

互补关系，不是同一个东西：

- **Tool**：结构化函数调用（有 schema、有返回值、模型生成 tool_call）
- **Skill**：自然语言指令包（prompt 注入，agent 读取后按指令操作，可能调用 tool）

Skill 可以引用 Tool（"Use the web_search tool to..."），但 Skill 本身不是 Tool。

---

## 5. agent-demo 实现建议

### MVP 需要做的

1. **Skill 目录扫描** — 找到所有 SKILL.md
2. **Frontmatter 解析** — 提取 name + description
3. **Prompt 注入** — 新增 SkillsSection，生成 XML 注入 system prompt
4. **read_file 工具** — agent 能读取 SKILL.md（激活层）和 scripts/references（资源层）
5. **多来源 + 优先级覆盖** — workspace > global > builtin

### MVP 不需要做的

- 安装机制（install spec）
- `/命令` 系统
- 资格检查（requires）
- prompt 大小限制保护（skill 少时不需要）
- always / disable-model-invocation 等高级策略
- Skill 同步（sandbox）

### 需要新增的组件

- `read_file` 工具 — agent 读取文件（Skill 激活 + 资源访问都依赖它）
- `SkillsLoader` — 扫描 + 解析 + 去重
- `SkillsSection` — ContextBuilder 的新 PromptSection
