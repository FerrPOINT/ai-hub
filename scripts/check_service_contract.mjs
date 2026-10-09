/** Offline design vector only. Runtime verifier belongs to the planned Rust adapter. */
import {
  createHash,
  createPrivateKey,
  createPublicKey,
  sign,
  verify,
} from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const domain = Buffer.from("AIHUB-SERVICE-INFERENCE-V1\0");
// RFC 8032 public test seed. It is never a deployment trust key.
const seed = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
const privateKey = createPrivateKey({
  key: Buffer.from("302e020100300506032b657004220420" + seed, "hex"),
  format: "der",
  type: "pkcs8",
});
const publicKey = createPublicKey(privateKey),
  publicDer = publicKey.export({ format: "der", type: "spki" }).toString("hex");
const uuid = (n) =>
  `${n.toString(16).padStart(8, "0")}-0000-4000-8000-000000000001`;
const namespace = { registry_instance_id: uuid(1), namespace_id: uuid(2) };
const body = {
  protocol: "chat",
  execution_context: {
    schema_version: 2,
    operation_id: uuid(3),
    namespace,
    task: { tracker_instance_id: uuid(4), task_id: uuid(5) },
    repositories: [{ forge_instance_id: uuid(6), repository_id: uuid(7) }],
  },
  invocation: {
    model: "main-dev",
    messages: [{ role: "user", content: "Синтетическая проверка" }],
    stream: false,
  },
};
const bytes = Buffer.from(JSON.stringify(body)),
  digest = (b) => createHash("sha256").update(b).digest("hex");
const claims = {
  schema_version: 1,
  action: "infer",
  issuer_instance_id: uuid(8),
  audience_installation_id: uuid(9),
  machine_subject: "fleet:fixture",
  grant_id: uuid(10),
  client_id: uuid(11),
  request_id: uuid(12),
  context_operation_id: body.execution_context.operation_id,
  execution_id: uuid(13),
  profile_revision_id: "80000000-0000-4000-8000-000000000012",
  namespace,
  fencing_token: "5",
  issued_at: "2026-10-09T13:29:00Z",
  expires_at: "2026-10-09T13:35:00Z",
  lease_expires_at: "2026-10-09T13:34:00Z",
  request_body_sha256: digest(bytes),
  max_provider_cost: "0.05",
  currency: "USD",
};
function signed(c) {
  const payload = JSON.stringify(c);
  return {
    schema_version: 1,
    key_id: "rfc8032-design-only",
    payload,
    signature_hex: sign(
      null,
      Buffer.concat([domain, Buffer.from(payload)]),
      privateKey,
    ).toString("hex"),
  };
}
function accepts(
  envelope,
  rawBody,
  {
    now = "2026-10-09T13:30:00Z",
    revoked = false,
    fence = 5n,
    clientNamespace = namespace,
  } = {},
) {
  try {
    if (
      envelope.schema_version !== 1 ||
      envelope.key_id !== "rfc8032-design-only" ||
      Object.keys(envelope).sort().join() !==
        ["schema_version", "key_id", "payload", "signature_hex"].sort().join()
    )
      return false;
    if (
      !verify(
        null,
        Buffer.concat([domain, Buffer.from(envelope.payload)]),
        publicKey,
        Buffer.from(envelope.signature_hex, "hex"),
      )
    )
      return false;
    const c = JSON.parse(envelope.payload),
      b = JSON.parse(new TextDecoder("utf8", { fatal: true }).decode(rawBody));
    if (
      Object.keys(c).sort().join() !== Object.keys(claims).sort().join() ||
      c.action !== "infer" ||
      c.schema_version !== 1
    )
      return false;
    if (
      c.issuer_instance_id !== claims.issuer_instance_id ||
      c.audience_installation_id !== claims.audience_installation_id ||
      c.machine_subject !== claims.machine_subject ||
      c.client_id !== claims.client_id
    )
      return false;
    if (
      c.profile_revision_id !== claims.profile_revision_id ||
      c.request_body_sha256 !== digest(rawBody) ||
      c.context_operation_id !== b.execution_context.operation_id
    )
      return false;
    if (
      JSON.stringify(c.namespace) !== JSON.stringify(clientNamespace) ||
      JSON.stringify(c.namespace) !==
        JSON.stringify(b.execution_context.namespace)
    )
      return false;
    if (
      revoked ||
      BigInt(c.fencing_token) < fence ||
      BigInt(c.fencing_token) > 18446744073709551615n ||
      Date.parse(c.issued_at) > Date.parse(now) + 30000 ||
      Date.parse(now) >=
        Math.min(Date.parse(c.expires_at), Date.parse(c.lease_expires_at))
    )
      return false;
    return true;
  } catch {
    return false;
  }
}
const vector = {
  kind: "offline-design-vector",
  algorithm: "Ed25519",
  domain: "AIHUB-SERVICE-INFERENCE-V1 NUL",
  public_key_spki_hex: publicDer,
  raw_body_utf8: bytes.toString(),
  claims,
  envelope: signed(claims),
  runtime_acceptance: "not_run",
};
const path = resolve(root, "docs/examples/service-adapter-vector.json");
if (process.argv.includes("--write-vector"))
  writeFileSync(path, JSON.stringify(vector, null, 2) + "\n");
const stored = JSON.parse(readFileSync(path, "utf8"));
const checks = [
  [
    accepts(stored.envelope, Buffer.from(stored.raw_body_utf8)),
    "valid exact body",
  ],
  [
    !accepts(stored.envelope, Buffer.from(stored.raw_body_utf8 + " ")),
    "body bytes changed",
  ],
  [
    !accepts({ ...stored.envelope, signature_hex: "0".repeat(128) }, bytes),
    "bad signature",
  ],
  [
    !accepts(signed({ ...claims, audience_installation_id: uuid(15) }), bytes),
    "wrong audience",
  ],
  [!accepts(signed({ ...claims, client_id: uuid(16) }), bytes), "wrong client"],
  [
    !accepts(signed({ ...claims, context_operation_id: uuid(17) }), bytes),
    "wrong context operation",
  ],
  [
    !accepts(
      signed({
        ...claims,
        namespace: { ...namespace, namespace_id: uuid(18) },
      }),
      bytes,
    ),
    "wrong Namespace",
  ],
  [
    !accepts(signed({ ...claims, profile_revision_id: "12" }), bytes),
    "ordinal is not UUID revision",
  ],
  [
    !accepts(stored.envelope, bytes, { now: "2026-10-09T13:34:00Z" }),
    "expired lease",
  ],
  [!accepts(stored.envelope, bytes, { revoked: true }), "revoke tombstone"],
  [!accepts(stored.envelope, bytes, { fence: 6n }), "lower fence"],
  [
    !accepts({ ...stored.envelope, public_key_spki_hex: publicDer }, bytes),
    "caller cannot add trust key",
  ],
];
for (const [ok, name] of checks) if (!ok) throw Error(name);
console.log(
  `Signed service design vectors: PASS (${checks.length}); actual adapter/runtime NOT RUN`,
);
