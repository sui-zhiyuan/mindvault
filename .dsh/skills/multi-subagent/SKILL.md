---
name: multi-subagent
description: Use when you need to prove a delegation route works end to end — starts two continuable background subagents (one per provider/model), has each run a hello-world probe, then continues both with a second message to prove they are continuable. Use after adding or changing an LLM route, or whenever delegation on specific provider/model pairs must be evidenced rather than assumed.
whenToUse: After adding or changing a provider route (for example a new llm-pi-ai route in the profile patch), or whenever you need evidence that `subagent` provider/model selection, background execution and continuation all work.
---

# 可续期双子代理冒烟测试

用两个**可续期**（后台）子代理跑一次 hello world，验证三件事：两条子代理路由已注册、
子代理跑得通工具、并且**能续**（对同一个 id 再发一次 `send_message` 仍有响应）。

## 步骤

1. 先确认路由：调用 `list_subagent_models`（不带参数看 provider 列表；带 `provider` 看它的模型）。
   缺任何一条就**停下并原样报告**，不要猜 provider/model，也不要换成别的模型继续。

   | # | provider | model | 期望标记 |
   |---|---|---|---|
   | 1 | `opencode-gpt` | `gpt-6.1-sol` | `HELLO-WORLD-1` |
   | 2 | `deepseek-official` | `deepseek-v4-pro` | `HELLO-WORLD-2` |

2. **在同一条消息里**并行启动两个后台子代理（`subagent`；`run_in_background` 默认开启，不要关）。
   子代理不共享本会话上下文，所以每个 prompt 必须自包含，内容照抄下面这段，只替换 `<n>` 与 `<provider>`：

   > 这是一次 harness 冒烟测试，只做下面两件事，别做别的。
   > 1. 用 bash 工具执行：`echo hello-world`
   > 2. 只回两行，不要任何解释、不要代码块：
   >    `HELLO-WORLD-<n> OK: hello-world`
   >    `PROVIDER-TAG: <provider>`
   >
   > 规则：不要创建/修改/删除任何文件；不要跑其他命令。

3. 记下两个 subagent id，**不要阻塞等待**（它们是后台的）。

4. 两个都返回后，对**同一个 id** 各发一次 `send_message`：

   > 续期测试：只回一行 `CONTINUED-<n>`

   有回 → 续期能力 ✅；报错或超时将错误原文记录下来 → ❌

5. 汇总成一张表：`provider / model / 首轮应答 / 续期应答 / 结论`。

## 失败处理

任何一步失败都**原样引用**错误文本（例如 `MISSING_CREDENTIAL`、`UNKNOWN_MODEL`、403 响应体），
不要重试超过一次，不要改 provider/model 去"绕过"问题。

## 边界

- 本技能证明的是：路由已注册、能被 `subagent` 选中、能执行工具、能被 `send_message` 续期。
- 不证明：模型能力、长任务稳定性、延迟或成本。
- 只有确实不再需要某个子代理时才 `interrupt_agent`。
- 换成其他路由跑同一套（例如 Zen 上的 `claude-opus-5-5`）时，只需要改上面的路由表与标记。
