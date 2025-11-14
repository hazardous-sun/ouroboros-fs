# Chapter 3: Ring Topology & Discovery

In the [previous chapter](02_node_.md), we learned that each **Node** is like a person in a circle, and each person knows who is sitting to their immediate right (their `next_port`). This is great for passing simple messages one by one.

But what if a [Node](02_node_.md) needs a bird's-eye view? What if it needs to know everyone who is sitting at the table, not just its immediate neighbor?

This is where Ring Topology & Discovery comes in. It's the process OuroborosFS uses to map out its own network structure.

## What Problem Does This Solve?

Imagine you've just started three separate [Node](02_node_.md) instances on your computer. You've told Node 1 to connect to Node 2, Node 2 to connect to Node 3, and Node 3 to connect back to Node 1. They have formed a complete ring.

**Our main use case: How can we ask one Node, "Who else is in this network?" and get a complete map of all the connections?**

Knowing the full layout, or "topology," is essential for advanced operations. For example, if we want to store a file by splitting it into three pieces, we first need to know that there are three nodes available.

## The Sign-In Sheet Analogy

The process of discovery in OuroborosFS works just like passing a sign-in sheet around a circular table.

1.  **Start:** The first person (our "start node") takes the sheet. They write down their own name and the name of the person they're passing it to. For example: `NodeA -> NodeB`. They then pass the sheet to NodeB.
2.  **Pass It On:** NodeB receives the sheet. They see `NodeA -> NodeB`. They add their own link to the chain: `NodeB -> NodeC`. The sheet now says `NodeA -> NodeB; NodeB -> NodeC`. They pass it to NodeC.
3.  **Complete the Circle:** This continues until the sheet arrives back at the original person, NodeA. NodeA receives the sheet and sees the complete list of connections: `NodeA -> NodeB; NodeB -> NodeC; NodeC -> NodeA`.

Now, NodeA has a full map of the seating arrangement. The discovery is complete!

## The `TOPOLOGY WALK` Command

In OuroborosFS, this "sign-in sheet" is a special message initiated by the `TOPOLOGY WALK` command. A user or the [Gateway](01_gateway_.md) can send this command to any [Node](02_node_.md) in the ring to kick off the discovery process.

Let's see how this works step-by-step.

### A Diagram of the Walk

Imagine we have three nodes running: 7001, 7002, and 7003. A client asks Node 7001 to start a topology walk.

```mermaid
sequenceDiagram
    participant Client
    participant Node7001 as Node 7001 (Start)
    participant Node7002 as Node 7002
    participant Node7003 as Node 7003

    Client->>Node7001: TOPOLOGY WALK
    Note over Node7001: I'll start the walk. History: "7001->7002"
    Node7001->>Node7002: TOPOLOGY HOP (history: "7001->7002")

    Note over Node7002: I'll add my link. History: "7001->7002;7002->7003"
    Node7002->>Node7003: TOPOLOGY HOP (history: "...;7002->7003")

    Note over Node7003: I'll add my link. My next hop is the start node!
    Node7003->>Node7001: TOPOLOGY DONE (final history: "...;7003->7001")

    Note over Node7001: The walk is complete! I have the full map.
    Node7001-->>Client: Full Topology List
```

The process uses three related commands:
*   `TOPOLOGY WALK`: Sent by a client to start the process.
*   `TOPOLOGY HOP`: The "sign-in sheet" message passed from one [Node](02_node_.md) to the next.
*   `TOPOLOGY DONE`: The final message sent by the last [Node](02_node_.md) back to the start [Node](02_node_.md) to complete the walk.

## A Glimpse into the Code

Let's look at the Rust code that powers this process. We'll simplify it to focus on the key logic.

### 1. Starting the Walk

When a [Node](02_node_.md) receives `TOPOLOGY WALK`, it becomes the "start node". It prepares the initial "sign-in sheet" and sends the first `HOP` message.

**File:** `src/server.rs`
```rust
async fn handle_topology_walk<W: AsyncWrite + Unpin>(
    node: &Node,
    writer: &mut W,
) -> Result<(), AnyErr> {
    // Create a unique tracking number for this walk
    let token = node.make_walk_token();

    // Create the first entry on the "sign-in sheet"
    let Some(history) = node.first_walk_history().await else {
        // ... handle error if this node doesn't have a neighbor ...
    };

    // Send the first HOP message to our neighbor
    node.forward_topology_hop(&token, &node.port, &history).await?;

    // ... wait for the walk to complete and then respond to the client ...
}
```
This code does two things:
1.  It creates a `token` (like `7001-1`) to uniquely identify this specific walk.
2.  It creates the first piece of `history` (e.g., `"7001->7002"`) and sends it to its neighbor.

### 2. Handling a Hop

When a [Node](02_node_.md) receives a `TOPOLOGY HOP` message, it adds its own information and decides whether to continue the walk or finish it.

**File:** `src/server.rs`
```rust
async fn handle_topology_hop(/*...*/) -> Result<(), AnyErr> {
    // Get the address of my neighbor
    let Some(next_addr) = node.get_next().await else { /* ... */ };

    // Add my link to the history: "prev_history;my_port->next_port"
    let new_history = append_edge(history, &node.port, &next_addr);

    // Is my neighbor the node that started this walk?
    if port_str(&next_addr) == port_str(&start_addr) {
        // Yes! The circle is complete. Send DONE back to the start.
        node.send_topology_done(&start_addr, &token, &new_history).await?;
    } else {
        // No. Forward the HOP to my neighbor.
        node.forward_topology_hop(&token, &start_addr, &new_history).await?;
    }

    // ... respond OK to the node that sent me the hop ...
}
```
This is the core logic of the walk. Each [Node](02_node_.md) simply appends its own "A -> B" connection and checks if B is the starting point. If it is, the walk is over. If not, it passes the sheet along.

The `append_edge` function is a simple helper to add a new connection to the history string.

**File:** `src/node.rs`
```rust
pub fn append_edge(mut history: String, from_addr: &str, to_addr: &str) -> String {
    let from = port_str(from_addr); // e.g., "127.0.0.1:7001" -> "7001"
    let to = port_str(to_addr);
    let edge = format!("{from}->{to}");

    if history.is_empty() {
        edge
    } else {
        history.push(';');
        history.push_str(&edge);
        history
    }
}
```

### 3. Finishing the Walk

Finally, the start [Node](02_node_.md) receives the `TOPOLOGY DONE` message. It now has the complete map.

**File:** `src/server.rs`
```rust
async fn handle_topology_done(/*...*/) -> Result<(), AnyErr> {
    // 1. Signal the original client that the walk is finished
    //    and give it the final history string.
    node.finish_walk(&token, history.clone()).await;

    // 2. Store the complete map for our own use.
    node.set_topology_from_history(&history).await;

    // 3. (Bonus) Broadcast this new map to all other nodes
    //    so everyone has the same view of the network.
    tokio::spawn(async move {
        node_clone.broadcast_topology_set().await;
    });

    // ... respond OK ...
}
```
This final step is crucial. The start [Node](02_node_.md) not only completes the request but also updates its own internal state (`topology_map`) and shares this complete picture with every other [Node](02_node_.md) in the ring. After a successful walk, every [Node](02_node_.md) knows the full network topology.

## Conclusion

You've just learned how OuroborosFS discovers its own structure.

*   The system forms a **logical ring** where each [Node](02_node_.md) knows its neighbor.
*   A `TOPOLOGY WALK` command initiates a discovery process, much like **passing a sign-in sheet** around a table.
*   A `HOP` message travels the ring, **collecting connection information** at each step.
*   When the message returns to the start, the **full network map is revealed** and shared with all nodes.

This discovery process is the foundation for many of OuroborosFS's powerful features. Now that we know how many nodes are in our network and how they're connected, we can finally start using them to store data.

In the next chapter, we'll explore how OuroborosFS takes a large file, breaks it into pieces, and distributes it across the ring.

➡️ **Next Chapter: [File Distribution & Chunking](04_file_distribution___chunking_.md)**

---

Generated by [AI Codebase Knowledge Builder](https://github.com/The-Pocket/Tutorial-Codebase-Knowledge)