// Which `result` message becomes a round and which becomes a named fault. The
// mapping is the sidecar's whole judgement of a finished query, so it is
// asserted directly against the exported decision, with no credential, no
// network, and no `query()`.

import assert from "node:assert/strict";
import test from "node:test";
import { apiRetryReason, resultFault, resultReason, SidecarFault } from "../src/main.ts";

function reasonOf(message: Record<string, unknown>): string | null {
  const fault = resultFault(message);
  if (fault === null) {
    return null;
  }
  assert.ok(fault instanceof SidecarFault);
  return fault.reason;
}

test("a spent turn cap is an ordinary output, not a fault", () => {
  // The cap is the lane's own remaining round budget, so reaching it means the
  // model spent turns the evaluator would also have allowed.
  assert.equal(resultFault({ subtype: "error_max_turns", num_turns: 24 }), null);
  assert.equal(resultFault({ subtype: "success" }), null);
});

test("every other result subtype carries its named fault", () => {
  assert.equal(reasonOf({ subtype: "error_max_budget_usd" }), "max_budget_usd");
  assert.equal(reasonOf({ subtype: "error_during_execution" }), "execution_error");
  assert.equal(
    reasonOf({ subtype: "error_max_structured_output_retries" }),
    "execution_error",
  );
  assert.equal(reasonOf({ subtype: "error_nobody_has_named_yet" }), "unknown");
});

test("an API status outranks the subtype it arrived on", () => {
  assert.equal(
    reasonOf({ subtype: "error_during_execution", api_error_status: 429 }),
    "rate_limited",
  );
  assert.equal(
    reasonOf({ subtype: "error_during_execution", api_error_status: 503 }),
    "server_error",
  );
  assert.equal(
    reasonOf({ subtype: "error_during_execution", api_error_status: 400 }),
    "execution_error",
  );
  assert.equal(resultReason("error_during_execution", "429"), "execution_error");
});

test("a retry category names the fault when the result carries one", () => {
  const categories: [string, string][] = [
    ["rate_limit", "rate_limited"],
    ["overloaded", "overloaded"],
    ["server_error", "server_error"],
    ["authentication_failed", "authentication_failed"],
    ["oauth_org_not_allowed", "org_not_allowed"],
    ["billing_error", "billing_error"],
    ["invalid_request", "invalid_request"],
    ["model_not_found", "model_not_found"],
    ["max_output_tokens", "max_output_tokens"],
    ["something_new", "unknown"],
  ];
  for (const [category, reason] of categories) {
    assert.equal(apiRetryReason(category), reason, category);
    assert.equal(
      reasonOf({ subtype: "error_during_execution", error: category }),
      reason,
      category,
    );
  }
});

test("a fault's detail carries the subtype and the reported errors", () => {
  const fault = resultFault({
    subtype: "error_during_execution",
    errors: ["the pipe closed", "and stayed closed"],
  });
  assert.ok(fault instanceof SidecarFault);
  assert.equal(fault.reason, "execution_error");
  assert.equal(fault.message, "error_during_execution: the pipe closed; and stayed closed");
});
