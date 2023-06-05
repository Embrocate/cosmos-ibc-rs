# ADR 007: LIGHT CLIENT DEPENDENCIES

## Context

This ADR is meant to address the main limitation of our current light client API, first introduced in [ADR 4] and [later improved] to adopt some of the ideas present in ibc-go's [ADR 6]. Implementing some `ClientState` methods require additional information from the host. For example, the Tendermint client's implementation of `ClientState::verify_client_message` needs [access to the host timestamp] to properly perform a message's verification. Previously, we solved this problem by [giving a reference] to a `ValidationContext` and `ExecutionContext`, since most methods are already made available by these traits. However, this solution has some limitations:

1. Not all methods needed by every future light client is present in `ValidationContext` or `ExecutionContext`. For example, if a light client X finds that it would need access to some resource X, currently the only way to solve this is to submit a PR on the ibc-rs repository that adds a method `get_resource_Y()` to `ValidationContext`.
    + This means that every host will need to implement `get_resource_Y()`, even if they don't use light client X.
    + It clutters up `ValidationContext` and `ExecutionContext`.
2. We found that some methods only needed by the Tendermint light client made their way into `ValidationContext`.
    + `next_consensus_state()` and `prev_consensus_state()` are not used in the core handlers; they're only there because of the Tendermint light client.
3. It gives more power to light clients than they really need
    + By giving the light clients access to `ValidationContext` and `ExecutionContext`, we're effectively giving them the same capabilities as the core handlers.
    + Although our current model is that all code is trusted (including light clients we didn't write), restraining the capabilities we give to light clients at the very least eliminates a class of bugs (e.g. calling the wrong method), and serves as documentation for exactly what the light client will need.

This ADR is all about fixing this issue; namely, to enable light clients to impose a `Context` trait for the host to implement. We loosely say that the light client "specifies dependencies on the host".

[ADR 4]: ../architecture/adr-004-light-client-crates-extraction.md
[later improved]: https://github.com/cosmos/ibc-rs/pull/584
[ADR 6]: https://github.com/cosmos/ibc-go/blob/main/docs/architecture/adr-006-02-client-refactor.md
[access to the host timestamp]: https://github.com/cosmos/ibc-rs/blob/3e2566b3102af3fb6185cdc158cff818ec605535/crates/ibc/src/clients/ics07_tendermint/client_state/update_client.rs#L70
[giving a reference]: https://github.com/cosmos/ibc-rs/blob/3e2566b3102af3fb6185cdc158cff818ec605535/crates/ibc/src/core/ics02_client/client_state.rs#L72

## Decision

The primary change is that we will no longer use dynamic dispatch. Namely, we will remove all occurances of `dyn ValidationContext`, `Box<dyn ConsensusState>`, etc. This is because our solution will be centered around generics, and our traits will no longer be trait object safe.

### Changes to `ClientState`

The `ClientState` functionality is split into 4 traits: 
+ `ClientStateBase`, 
+ `ClientStateInitializer<AnyConsensusState>`, 
+ `ClientStateValidation<ClientValidationContext>`, and 
+ `ClientStateExecution<ClientExecutionContext>`

Then, `ClientState` is defined as

```rust
pub trait ClientState<AnyConsensusState, ClientValidationContext, ClientExecutionContext>:
    ClientStateBase
    + ClientStateInitializer<AnyConsensusState>
    + ClientStateValidation<ClientValidationContext>
    + ClientStateExecution<ClientExecutionContext>
    // + ...
{
}
```

A blanket implementation implements `ClientState` when these 4 traits are implemented on a type. For details as to why `ClientState` was split into 4 traits, see the section "Why there are 4 `ClientState` traits".

The `ClientStateValidation` and `ClientStateExecution` are the most important ones, as they are the ones that enable light clients to specify dependencies on the host. Below, we discuss `ClientStateValidation`; `ClientStateExecution` works analogously.

 Say a light client needs a `get_resource_Y()` method from the host in `ClientState::verify_client_message()`. Then, they would first define a trait for the host to implement.

```rust
trait MyClientValidationContext {
    fn get_resource_Y(&self) -> Y;
}
```

Then, they would implement the `ClientStateValidation<ClientValidationContext>` trait *conditioned on* `ClientValidationContext` implementing `MyClientValidationContext`.

```rust
impl<ClientValidationContext> ClientStateValidation<ClientValidationContext> for MyClientState
where
    ClientValidationContext: MyClientValidationContext,
{
    fn verify_client_message(
        &self,
        ctx: &ClientValidationContext,
        // ...
    ) -> Result<(), ClientError> { 
        // `get_resource_Y()` accessible through `ctx`
    }

    // ...
}
```

This is the core idea of this ADR. Everything else is a consequence of wanting to make this work.

### Changes to `ValidationContext` and `ExecutionContext`

`ValidationContext` is now defined as:

```rust
pub trait ValidationContext: Router {
    type ClientValidationContext;
    type ClientExecutionContext;
    type AnyConsensusState: ConsensusState<EncodeError = ContextError>;
    type AnyClientState: ClientState<
        Self::AnyConsensusState,
        Self::ClientValidationContext,
        Self::ClientExecutionContext,
    >;

    // ...
}
```

`AnyConsensusState` and `AnyClientState` are expected to be enums that hold the consensus states and client states of all supported light clients. For example,

```rust
enum AnyConsensusState {
    Tendermint(TmConsensusState),
    Near(NearConsensusState),
    // ...
}

enum AnyClientState {
    Tendermint(TmClientState),
    Near(NearClientState),
    // ...
}
```

`ClientValidationContext` and `ClientExecutionContext` correspond to the same types described in the previous section. The host must ensure that these 2 types implement the Tendermint and Near "dependency traits" (as discussed in the previous section). For example,

```rust
struct MyClientValidationContext;

impl TmClientValidationContext for MyClientValidationContext {
    // ...
}

impl NearClientValidationContext for MyClientValidationContext {
    // ...
}
```

### `ClientState` and `ConsensusState` convience derive macros
Notice that `ValidationContext::AnyClientState` needs to implement `ClientState`, and `ValidationContext::AnyConsensusState` needs to implement `ConsensusState`. Given that `AnyClientState` and `AnyConsensusState` are enums that wrap types that implement `ClientState` or `ConsensusState` (respectively), implementing these traits is gruesome boilerplate:

```rust
impl ClientStateBase for AnyClientState {
    fn client_type(&self) -> ClientType {
        match self {
            Tendermint(cs) => cs.client_type(),
            Near(cs) => cs.client_type()
        }
    }

    // ...
}
```

To relieve users of such torture, we provide derive macros that do just that:

```rust
#[derive(ConsensusState)]
enum AnyConsensusState {
    Tendermint(TmConsensusState),
    Near(NearConsensusState),
    // ...
}

#[derive(ClientState)]
#[generics(consensus_state = AnyConsensusState,
           client_validation_context = MyClientValidationContext,
           client_execution_context = MyClientExecutionContext)
]
enum AnyClientState {
    Tendermint(TmClientState),
    Near(NearClientState),
    // ...
}
```

## FAQs

### Why there are 4 `ClientState` traits

The `ClientState` trait is defined as

```rust
trait ClientState<AnyConsensusState, ClientValidationContext, ClientExecutionContext>
```

The problem with defining all methods directly under `ClientState` is that it would force users to use fully qualified notation to call any method.

This arises from the fact that no method uses all 3 generic parameters. [This playground] provides an explanatory example. Hence, our solution is to have all methods in a trait use every generic parameter of the trait to avoid this problem.

[This playground]: https://play.rust-lang.org/?version=stable&mode=debug&edition=2021&gist=da65c22f1532cecc9f92a2b7cb2d1360

### Why have `ClientValidationContext` and `ClientExecutionContext` as opposed to just one `ClientContext`

### Alternatives to writing our own `ClientState` and `ConsensusState` derive macros
+ `enum_derive` and `enum_delegate`

## Consequences

> This section describes the consequences, after applying the decision. All consequences should be summarized here, not just the "positive" ones.

### Positive

### Negative
+ If 2 light clients need the same (or very similar) methods, then the host will need to reimplement the same method multiple times
    + Although mitigatable by implementing once in a function and delegating all trait methods to that implementation, it is at the very least additional boilerplate
+ Increased complexity.
+ Harder to document. 
    + Specifically, we do not write any trait bounds on the `Client{Validation, Execution}Context` generic parameters. The effective trait bounds are spread across all light client implementations that a given host uses.


### Neutral

## References

> Are there any relevant PR comments, issues that led up to this, or articles referenced for why we made the given design choice? If so link them here!

* [Main issue](https://github.com/cosmos/ibc-rs/issues/296)