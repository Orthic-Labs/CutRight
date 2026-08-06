#!/usr/bin/env python3
"""scripts/schema-check.py — stdlib-only JSON Schema (draft-07 subset) validator.

Usage: python3 scripts/schema-check.py SCHEMA.json INSTANCE.json [INSTANCE2.json ...]

Supported keywords: type, required, properties, additionalProperties,
items, enum, const, pattern, minLength, minItems, $ref (local
#/definitions/...), allOf, if/then/else, not. Unknown keywords are ignored.
Exit 0 when every instance validates, 1 otherwise.
"""

from __future__ import annotations

import json
import re
import sys

TYPE_MAP = {
    "object": dict,
    "array": list,
    "string": str,
    "boolean": bool,
    "null": type(None),
}


def _type_ok(instance, expected: str) -> bool:
    if expected == "integer":
        return isinstance(instance, int) and not isinstance(instance, bool)
    if expected == "number":
        return isinstance(instance, (int, float)) and not isinstance(instance, bool)
    py = TYPE_MAP.get(expected)
    if py is None:
        raise ValueError(f"unsupported schema type: {expected!r}")
    return isinstance(instance, py)


def validate(instance, schema: dict, root: dict, path: str = "$") -> list[str]:
    errors: list[str] = []
    if not isinstance(schema, dict):
        return errors

    if "$ref" in schema:
        ref = schema["$ref"]
        if not ref.startswith("#/"):
            raise ValueError(f"unsupported $ref: {ref}")
        node = root
        for part in ref[2:].split("/"):
            node = node[part]
        return validate(instance, node, root, path)

    if "const" in schema and instance != schema["const"]:
        errors.append(f"{path}: expected const {schema['const']!r}, got {instance!r}")

    if "enum" in schema and instance not in schema["enum"]:
        errors.append(f"{path}: {instance!r} not in enum {schema['enum']!r}")

    if "not" in schema and not validate(instance, schema["not"], root, path):
        errors.append(f"{path}: value {instance!r} matches a forbidden 'not' schema")

    if "type" in schema:
        expected = schema["type"]
        types = expected if isinstance(expected, list) else [expected]
        if not any(_type_ok(instance, t) for t in types):
            errors.append(f"{path}: expected type {expected}, got {type(instance).__name__}")
            return errors

    if isinstance(instance, str):
        if "pattern" in schema and not re.search(schema["pattern"], instance):
            errors.append(f"{path}: {instance!r} does not match pattern {schema['pattern']!r}")
        if "minLength" in schema and len(instance) < schema["minLength"]:
            errors.append(f"{path}: shorter than minLength {schema['minLength']}")

    if isinstance(instance, list):
        if "minItems" in schema and len(instance) < schema["minItems"]:
            errors.append(f"{path}: fewer than {schema['minItems']} items")
        item_schema = schema.get("items")
        if isinstance(item_schema, dict):
            for i, item in enumerate(instance):
                errors.extend(validate(item, item_schema, root, f"{path}[{i}]"))

    if isinstance(instance, dict):
        for req in schema.get("required", []):
            if req not in instance:
                errors.append(f"{path}: missing required property {req!r}")
        props = schema.get("properties", {})
        extra = set(instance) - set(props)
        if schema.get("additionalProperties") is False and extra:
            for key in sorted(extra):
                errors.append(f"{path}: unknown property {key!r}")
        for key, sub in props.items():
            if key in instance:
                errors.extend(validate(instance[key], sub, root, f"{path}.{key}"))

    for sub in schema.get("allOf", []):
        errors.extend(validate(instance, sub, root, path))

    if "if" in schema:
        if validate(instance, schema["if"], root, path):
            # 'if' schema not satisfied -> apply else branch when present.
            branch = schema.get("else")
        else:
            branch = schema.get("then")
        if branch:
            errors.extend(validate(instance, branch, root, path))

    return errors


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__, file=sys.stderr)
        return 2
    with open(argv[1], encoding="utf-8") as fh:
        root = json.load(fh)
    failed = False
    for instance_path in argv[2:]:
        with open(instance_path, encoding="utf-8") as fh:
            instance = json.load(fh)
        errors = validate(instance, root, root)
        if errors:
            failed = True
            print(f"FAIL {instance_path}", file=sys.stderr)
            for err in errors:
                print(f"  {err}", file=sys.stderr)
        else:
            print(f"OK   {instance_path}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
