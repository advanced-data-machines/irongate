"""Small process-based comparison contract; protocol knowledge stays in suites."""

from dataclasses import dataclass, field
import hashlib
from typing import Callable

from measure import probe


@dataclass(frozen=True)
class Case:
    name: str
    mode: str
    data: bytes
    code: int = 53
    expected: dict = field(default_factory=dict)


@dataclass(frozen=True)
class Workload:
    name: str
    mode: str
    data: bytes
    code: int = 53
    cross_implementation_comparable: bool = False
    caveat: str = "No matched-work comparison established."


@dataclass(frozen=True)
class Suite:
    name: str
    version: int
    description: str
    cases: Callable
    compare: Callable
    workloads: Callable
    memory_workloads: Callable = tuple


def require(condition, message):
    # Keep verification active under python -O as well as ordinary invocation.
    if not condition:
        raise AssertionError(message)


def execute(suite, binaries, invoke=probe):
    result = {"name": suite.name, "version": suite.version, "description": suite.description,
              "cases": [], "benchmarks": {}, "memory": {}}
    names = set()
    for case in suite.cases():
        require(case.name not in names, f"Duplicate case: {suite.name}/{case.name}")
        names.add(case.name)
        responses = {name: invoke(binary, case.mode, case.data, case.code)[0]
                     for name, binary in binaries.items()}
        detail = suite.compare(case, responses["c"], responses["sanitized"], responses["rust"])
        result["cases"].append({"name": case.name, "mode": case.mode, "code": case.code,
                                "input_sha256": hashlib.sha256(case.data).hexdigest(),
                                "expected": case.expected, "sanitizer_passed": True, **detail})
    require(bool(names), f"Empty comparison suite: {suite.name}")
    result["result"] = "passed_with_documented_differences"
    return result
