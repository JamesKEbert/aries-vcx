# VCX Framework Architecture Documentation

## Framework Structure

The VCX Framework is comprised of `Modules`, `Services`, and `Repositories`:

`Module` -- a collection of functions that perform certain behaviors, such as "connect", "accept credential", or "create DID". Contains no in-memory or persistant data storage. Must be passed references to `Repositories` and `Services` as-needed. Is horizontally scalable--the only limiter being the registries/data stores themselves.

`Service` -- A `Module` that contains in-memory state, such as a transport references. Usage of a `Service` should be carefully considered, as having in-memory state makes horizontal scaling challenging, additionally, references makes code more interdependent, which can cause cyclical dependency issues, as well as make code harder to test. Also, ideally a service should not reference another service where possible for similar reasons as above. It can reference other modules, as that is an abstracted entry point that can be easily substituted via an interface/trait implementation or mocked in testing.

`Repository` -- a structure that persists long-term data (via DBs or other data stores) that can be accessed via CRUD type operations. The `Repository` will also broadcast record update events related to CRUD operationsl.

### Services

- Events Service -- a service that emits events and holds loose references to listeners, and so therefore must be a service.

> May not be necessary if individual registries / services manage their individual events. Additionally separating may reduce coupling between modules (increasing flexibility), but could introduce fragmentation is approach or implementation. -- additional thoughts: having the repository manage their individual events would require any event to have a corresponding state update, which for instance, errors may not have. Additionally, not all modules may have a repository where there is not persistant data being handled for the module in the framework. Additionally, the simplicity from the end-dev-consumer POV to work with an individual service for all events is appealing (especially given that it can be completely flexible in the data contained within the events). Additionally, the repository could also trigger events on CRUD operations via the Event Service, but is not the only place events can be triggered (events could be triggered via the module as well that are not CRUD specific, theoretically). - @JamesKEbert

- Transport Service -- a service that manages inbound / outbound transport handles -- HTTP handlers, WS managers, etc. These are references held in memory and therefore must be a service.
- Mediator Service -- a service that manages the pickup relationship with mediators as this is a long-running task.
  > Possibly might not need to be a service, will need implementation/additional thought to determine - @JamesKEbert

### Modules

- Messaging Module -- a module that provides the functionality for sending and receiving DIDComm messages, including encryption and decryption.
  > This potentially could require being a service due to HTTP response behaviors (needing to be able to send a message back in response to an HTTP message via DIDComm return route mechanisms rather than via the standard approach of a completely new outbound message) - @JamesKEbert
- Connection Module -- a module that cooridinates the behavior of connecting via DIDComm to another DIDComm agent.
- Mediation Module -- a module that cooridinates behaviors and relationships with mediators for the purposes of DIDComm connections and inbound message delivery.

### Repositories

- DID Repository -- holds all the data related to DIDs that we control and DIDs that have been shared with us (such as used during connections)
- Connections Repository -- holds the states of all connection records, including states, metadata, and references to DIDs
- Mediator Repository -- holds all the states of the mediator records, including references to associated connections.

## Sample inbound connection requ    #[derive(Error, Debug)]
    pub enum MessagingError {
        #[error("error resolving DID `{1}`")]
        DidResolution(#[source] GenericError, String),
        #[error("error resolving peer DID `{1}`")]
        DidResolutionPeerDid(#[source] DidPeerError, String),
        #[error("unable to get service from DIDDoc for DID `{1}`")]
        InvalidDidDocService(#[source] DidDocumentLookupError, String),
        #[error("error encrypting message")]
        EncryptMessage(#[source] AriesVcxError),
        #[error("error decrypting message")]
        DecryptMessage(#[source] AriesVcxError),
    }est message flow:

- Incoming HTTP POST message to HTTP server
  - handled by Transport Service
    - calls Messaging Module
      - calls relevant module handler, specifically the Connection Module
        - creates response message
          - stores/updates record in Connection Repository with created status
            - emits event via Events Service
          - sends response message via Messaging Module
            - call transport service to send message
              - call outbound HTTP handler to send message
          - stores/updates record in Connection Repository with sent status
            - emits event via Events Service

## Error Handling
### Use Specific errors

As framework developers, where possible, we should avoid making general-case errors, such as:

```rust
  #[derive(Error, Debug)]
  struct GeneralMessagingError {
    string: String
  }

  err(GeneralMessagingError::new(format!("Error resolving DID {did}")));
  err(GeneralMessagingError::new("Error sending message"));
```

Instead, prefer enums that provide the ability to `match` against, giving the end-developer complete flexibility in how to proceed:

```rust
  #[derive(Error, Debug)]
  enum MessagingError {
    #[error("error resolving DID `{1}`")]
    DidResolution(#[source] GenericError, String),
    #[error("error resolving peer DID `{1}`")]
    DidResolutionPeerDid(#[source] DidPeerError, String),
  }
```

### Avoid General Mapping Errors

As framework developers, we should avoid wrapping errors with `from`, such as:

```rust
  #[derive(Error, Debug)]
  enum MessagingError {
    #[error("Aries VCX Error")]
    DecryptMessage(#[from] AriesVcxError),
  }

  fn example() -> Result<(), MessagingError> {
    let value = function_that_returns_ariesvcxerror()?;
  }
```
or
```rust
impl From<serde_json::Error> for AriesVcxError {
  fn from(_err: serde_json::Error) -> Self {
    AriesVcxError::from_msg(AriesVcxErrorKind::InvalidJson, "Invalid json".to_string())
  }
}
```

Why? Well, the above makes it very easy to handle errors as *framework developers*, but reduces the info provided in errors for the *end-developer*. Additionally, errors can then be provided with additional information/context that is not available in the source error. 

Instead, prefer mapping the error, which is illustrated well here with two errors that have different surrounding contexts but have the same underlying error:

```rust
  #[derive(Error, Debug)]
  pub enum MessagingError {
    #[error("error encrypting message")]
    EncryptMessage(#[source] AriesVcxError),
    #[error("error decrypting message")]
    DecryptMessage(#[source] AriesVcxError),
  }

  fn example() -> Result<(), MessagingError> {
    let value = function_that_returns_ariesvcxerror().map_err(MessagingError::EncryptMessage)?;
  }
```