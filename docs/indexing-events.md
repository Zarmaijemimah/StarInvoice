# Indexing StarInvoice Events

StarInvoice emits Soroban contract events at every state transition. This guide explains the event topic structure and shows how to subscribe to them using the Stellar SDK.

## Event topic structure

Every event published by `events.rs` uses a two-symbol topic tuple followed by a data payload:

| Function           | Topic                        | Data payload                              |
|--------------------|------------------------------|-------------------------------------------|
| `create_invoice`   | `("INVOICE", "created")`     | `(invoice_id, freelancer, client, amount)`|
| `fund_invoice`     | `("INVOICE", "funded")`      | `(invoice_id, client, amount)`            |
| `mark_delivered`   | `("INVOICE", "delivered")`   | `(invoice_id, freelancer)`                |
| `approve_payment`  | `("INVOICE", "approved")`    | `(invoice_id, client)`                    |
| `cancel_invoice`   | `("INVOICE", "cancelled")`   | `(invoice_id, cancelled_by)`              |
| `release_payment`  | `("INVOICE", "released")`    | `(invoice_id, freelancer, amount)`        |

Topics are encoded as `ScSymbol` values on-chain. When querying via Horizon or RPC, they appear as base64-encoded XDR or as decoded symbol strings depending on the client library version.

## Approach 1 — Horizon event streaming

Horizon exposes a `/transactions` and `/effects` stream, but for Soroban contract events the recommended approach is the **Soroban RPC `getEvents` method**.

```typescript
import { SorobanRpc, xdr, StrKey } from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");

const CONTRACT_ID = "C..."; // your deployed contract address

async function fetchInvoiceEvents(startLedger: number) {
  const response = await server.getEvents({
    startLedger,
    filters: [
      {
        type: "contract",
        contractIds: [CONTRACT_ID],
        // Filter to only "INVOICE" events — omit `topics` to get all contract events
        topics: [["INVOICE"]],
      },
    ],
  });

  for (const event of response.events) {
    const [namespace, action] = event.topic.map((t) =>
      xdr.ScVal.fromXDR(t, "base64").sym().toString()
    );
    console.log(`[${namespace}:${action}]`, event.value);
  }
}

fetchInvoiceEvents(100); // replace with your deployment ledger
```

## Approach 2 — filtering by specific action

To subscribe only to `funded` events (e.g. to trigger a payment confirmation UI):

```typescript
const response = await server.getEvents({
  startLedger,
  filters: [
    {
      type: "contract",
      contractIds: [CONTRACT_ID],
      topics: [["INVOICE", "funded"]],
    },
  ],
});

for (const event of response.events) {
  // Decode the data payload: (invoice_id: u64, client: Address, amount: i128)
  const data = xdr.ScVal.fromXDR(event.value, "base64").vec();
  if (!data) continue;

  const invoiceId = data[0].u64().toString();
  const client = StrKey.encodeEd25519PublicKey(
    data[1].address().accountId().ed25519()
  );
  const amount = data[2].i128().toString();

  console.log(`Invoice ${invoiceId} funded by ${client} for ${amount} stroops`);
}
```

## Approach 3 — custom off-chain indexer

For analytics or frontend filtering (e.g. `get_invoices_by_amount_range`), a lightweight off-chain indexer avoids the on-chain full-scan cost entirely.

A minimal Node.js indexer pattern:

```typescript
import { SorobanRpc, xdr } from "@stellar/stellar-sdk";

const server = new SorobanRpc.Server("https://soroban-testnet.stellar.org");

// In-memory store — replace with a database for production
const invoices: Record<string, { amount: bigint; status: string }> = {};

async function syncEvents(fromLedger: number) {
  const res = await server.getEvents({
    startLedger: fromLedger,
    filters: [{ type: "contract", contractIds: [CONTRACT_ID] }],
  });

  for (const event of res.events) {
    const action = xdr.ScVal.fromXDR(event.topic[1], "base64").sym().toString();
    const data = xdr.ScVal.fromXDR(event.value, "base64").vec();
    if (!data) continue;

    const invoiceId = data[0].u64().toString();

    if (action === "created") {
      invoices[invoiceId] = { amount: data[3].i128(), status: "Pending" };
    } else if (action === "funded") {
      if (invoices[invoiceId]) invoices[invoiceId].status = "Funded";
    } else if (action === "delivered") {
      if (invoices[invoiceId]) invoices[invoiceId].status = "Delivered";
    } else if (action === "approved") {
      if (invoices[invoiceId]) invoices[invoiceId].status = "Approved";
    } else if (action === "released") {
      if (invoices[invoiceId]) invoices[invoiceId].status = "Completed";
    } else if (action === "cancelled") {
      if (invoices[invoiceId]) invoices[invoiceId].status = "Cancelled";
    }
  }
}

// Query locally — no on-chain scan needed
function getByAmountRange(min: bigint, max: bigint) {
  return Object.entries(invoices).filter(
    ([, inv]) => inv.amount >= min && inv.amount <= max
  );
}
```

## Further reading

- [Soroban RPC `getEvents` reference](https://developers.stellar.org/docs/data/rpc/api-reference/methods/getEvents)
- [stellar-sdk SorobanRpc docs](https://stellar.github.io/js-stellar-sdk/SorobanRpc.html)
- [Soroban event structure (XDR)](https://developers.stellar.org/docs/learn/smart-contract-internals/events)
