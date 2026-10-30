# Conversational intent comparison — October 29, 2026

The reported failure was real. The original request omitted the traveler's opening from dialogue history, put previous turns inside a single JSON user message, and included exact authored repetition/confession responses on every turn. It explicitly required an admission when confronted after secondary inspection. The server retained 30 exchanges, so truncation did not explain the first few turns in the screenshots.

## Controlled comparison

Twenty fixed officer utterances were used for each model: twelve with Cal, including the screenshot's wording questions, identity, an apology, an unsupported accusation and actual evidence confrontation; eight with honest Mara, including nervousness, an unknown family detail and a reference to the previous question. Each model generated its own conversation history. All used temperature 0.5 and 400 output tokens. Baseline and revised runs used the same captured scenario fixtures and questions. `scripts/benchmark_intent.py` reproduces the comparison; it is an opt-in paid utility.

| Model | Original JSON / valid emotion | Revised JSON / valid emotion | Original median | Revised median |
|---|---:|---:|---:|---:|
| Gemini 2.5 Flash-Lite | 20 / 18 of 20 | 17 / 16 of 20 | 0.57 s | 0.50 s |
| Gemini 2.5 Flash | 20 / 20 of 20 | 20 / 20 of 20 | 0.96 s | 1.05 s |
| GPT-4.1 mini | 20 / 20 of 20 | 20 / 20 of 20 | 1.11 s | 1.17 s |

These are single-run observations, not statistical model rankings. Raw trials did not apply the production repair loop, so invalid JSON/emotion counts remain visible. In the first revised batch, an invalid Flash-Lite emotion stopped that model's prompt exporter; Flash and GPT-4.1 mini completed. Flash-Lite was then rerun with invalid emotions recorded and normalized only for the next turn's context. Its 17 JSON / 16 valid-emotion result is that complete rerun. No failures were hidden as successes.

## Observed behavior

With the old prompt Flash-Lite answered “Why did you mention it was nothing commercial?” with the exact supplied tools joke. Gemini Flash and GPT-4.1 mini handled that reference better even before the rewrite. The problem was therefore both the restrictive scripted context and model capability.

With the revised context Gemini Flash answered the opening's wording question with “Just anticipating the question” / “trying to be efficient,” then reacted to the follow-up instead of listing cargo. It answered identity questions, softened after an apology, denied an unsupported accusation, and responded to evidence with argument and partial admission rather than the exact authored confession. Mara did not invent guilt when accused of smuggling; she could say she did not remember an unknown address and could refer back to the previous question.

The local configuration now uses Gemini 2.5 Flash. The live request uses proper user/assistant messages, includes the opening, excludes canned repeat/irrelevant/confession lines, and keeps every successful turn within the existing 40-attempt case budget. Cal now opens with an ordinary weekend greeting. Confessions are optional; the authoritative evidence and dispositions remain unchanged.

## Limits

Semantic repetition is not proven absent by JSON validation. Exact repeated lines still receive one repair attempt; the runtime does not falsely claim to validate arbitrary conversational meaning. Models can still invent spoken excuses or miss references. Smugglers are now allowed to invent verbal cover stories, but those cannot create physical evidence or change cargo, documents or guilt. Human playtesting remains necessary. Offline rehearsal remains a separate scripted mode and should not be used to judge generative conversation.
