"""Opt-in live OpenAI Responses API smoke probe. Never prints credentials."""
import json
import os
import urllib.request

key = os.environ.get("OPENAI_API_KEY")
if not key:
    raise SystemExit("BLOCKED: OPENAI_API_KEY is missing; live inference not verified")
body = json.dumps({
    "model": os.environ.get("OPENAI_MODEL", "gpt-5.1"),
    "input": "Reply with the single word READY.",
    "store": False,
    "max_output_tokens": 32,
}).encode("utf-8")
request = urllib.request.Request(
    "https://api.openai.com/v1/responses",
    data=body,
    method="POST",
    headers={"Authorization": "Bearer " + key, "Content-Type": "application/json"},
)
with urllib.request.urlopen(request, timeout=90) as response:
    result = json.load(response)
texts = [
    part.get("text", "")
    for item in result.get("output", [])
    for part in item.get("content", [])
    if part.get("type") == "output_text"
]
if not any(text.strip() for text in texts):
    raise SystemExit("FAIL: live inference returned no text")
print("PASS: OpenAI Responses API live inference")
