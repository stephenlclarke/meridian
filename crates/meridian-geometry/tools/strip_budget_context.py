"""Normalize only the explicit, fixed budget-context annotations for source checks."""
import re


def end_delimiter(text, start, opening="(", closing=")"):
    depth = 0
    quoted = False
    escaped = False
    for i in range(start, len(text)):
        character = text[i]
        if quoted:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                quoted = False
        elif character == '"':
            quoted = True
        elif character == opening:
            depth += 1
        elif character == closing:
            depth -= 1
            if depth == 0:
                return i
    raise AssertionError("missing delimiter")


def remove_statements(text, expression):
    while match := re.search(expression, text):
        opening = text.index("(", match.start())
        end = end_delimiter(text, opening) + 1
        end = text.index(";", end) + 1
        text = text[:match.start()] + text[end:]
    return text


def strip_context(text):
    text = text.replace("use crate::BudgetPhase;\n", "")
    text = remove_statements(text, r"\b(?:b|budget)\.(?:state|phase)\(")
    pattern = r"\b([a-z_]+)\.with_phase(?:_state)?\("
    while match := re.search(pattern, text):
        opening = text.index("(", match.start())
        end = end_delimiter(text, opening)
        closure = re.search(r"\|([a-z_]+)\|", text[opening:end])
        assert closure
        alias = closure[1]
        begin = opening + closure.end()
        body = text[begin:end].strip().removesuffix(",").strip()
        if body.startswith("{"):
            assert end_delimiter(body, 0, "{", "}") == len(body) - 1, repr(body)
            body = body[1:-1].strip()
        receiver = match[1]
        if alias != receiver:
            body = re.sub(rf"\b{alias}\b", receiver, body)
        text = text[:match.start()] + body + text[end+1:]
    return text


def body(text, signature):
    start = text.index(signature)
    opening = text.index("{", start)
    return text[start:end_delimiter(text, opening, "{", "}")+1]
