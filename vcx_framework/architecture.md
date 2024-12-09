# VCX Framework Architecture Documentation

## Framework Structure

The VCX Framework is comprised of `Modules`, `Services`, and `Registries`:

`Module` -- a collection of functions that perform certain behaviors, such as "connect", "accept credential", or "create DID". Contains no in-memory or persistant data storage. Must be passed references to `Registries` and `Services` as-needed. Is horizontally scalable--the only limiter being the registries/data stores themselves.

`Service` -- A `Module` that contains in-memory state, such as a transport references. Usage of a `Service` should be carefully considered, as having in-memory state makes horizontal scaling challenging, additionally, references makes code more interdependent, which can cause cyclical dependency issues, as well as make code harder to test. Also, ideally a service should not reference another service where possible for similar reasons as above. It can reference other modules, as that is an abstracted entry point that can be easily substituted via an interface/trait implementation or mocked in testing.

`Registry` -- a structure that persists long-term data (via DBs or other data stores) that can be accessed via CRUD type operations. The `Registry` will also broadcast record update events related to CRUD operationsl.

### Services

- Events Service -- a service that emits events and holds loose references to listeners, and so therefore must be a service.

> May not be necessary if individual registries / services manage their individual events. Additionally separating may reduce coupling between modules (increasing flexibility), but could introduce fragmentation is approach or implementation. -- additional thoughts: having the registry manage their individual events would require any event to have a corresponding state update, which for instance, errors may not have. Additionally, not all modules may have a registry where there is not persistant data being handled for the module in the framework. Additionally, the simplicity from the end-dev-consumer POV to work with an individual service for all events is appealing (especially given that it can be completely flexible in the data contained within the events). Additionally, the registry could also trigger events on CRUD operations via the Event Service, but is not the only place events can be triggered (events could be triggered via the module as well that are not CRUD specific, theoretically). - @jameskebert

- Transport Service -- a service that manages inbound / outbound transport handles -- HTTP handlers, WS managers, etc. These are references held in memory and therefore must be a service.
- Mediator Service -- a service that manages the pickup relationship with mediators as this is a long-running task.
  > Possibly might not need to be a service, will need implementation/additional thought to determine - @jameskebert

### Modules

- Messaging Module -- a module that provides the functionality for sending and receiving DIDComm messages, including encryption and decryption.
  > This potentially could require being a service due to HTTP response behaviors (needing to be able to send a message back in response to an HTTP message via DIDComm return route mechanisms rather than via the standard approach of a completely new outbound message) - @jameskebert
- Connection Module -- a module that cooridinates the behavior of connecting via DIDComm to another DIDComm agent.
- Mediation Module -- a module that cooridinates behaviors and relationships with mediators for the purposes of DIDComm connections and inbound message delivery.

### Registries

- DID registry -- holds all the data related to DIDs that we control and DIDs that have been shared with us (such as used during connections)
- Connections registry -- holds the states of all connection records, including states, metadata, and references to DIDs
- Mediator registry -- holds all the states of the mediator records, including references to associated connections.

## Sample inbound connection request message flow:

- Incoming HTTP POST message to HTTP server
  - handled by Transport Service
    - calls Messaging Module
      - calls relevant module handler, specifically the Connection Module
        - creates response message
          - stores/updates record in Connection Registry with created status
            - emits event via Events Service
          - sends response message via Messaging Module
            - call transport service to send message
              - call outbound HTTP handler to send message
          - stores/updates record in Connection Registry with sent status
            - emits event via Events Service

## Error Handling

What are VCX Framework's error handling goals:

- Specific - any given error must have a corresponding code/identifier such that an end user could provide the code and a developer could identify specifically where the issue occurred.
- Errors should be statically defined - **DON'T** allow generic error messages to be supplied, as it's much harder for end-developers to handle all error cases, as they cannot `Match()` errors by dynamic messages. **DO** define errors as `enum`s to enable matching
- The Framework should handle any dependency errors and then if appropriate provide a Framework specific error -- "Couldn't deserialize json" Serde dependency error gets handled/mapped and returned as "Couldn't process inbound DIDComm message" Framework error. Additionally, the original source of the error can be retrieved via .source(). This provides a powerful way of providing context to the original issue, without just blindly wrapping and passing all errors up the chain.

### Error Types:

- VCXFrameworkError -- A VCXFrameworkError is an error that generally is related to the action the end-developer was attempting to perform, such as connecting, issuance, etc ("Error connecting"). If a method or function is callable by end-developers, it should return a `VCXFrameworkError`. Given the open-ended, flexible nature of the framework, most `Modules` are likely to return `VCXFrameworkError`s over internal dependency errors.
- Dependency errors -- Dependency errors are errors stemming from dependencies, such as UUID, Serde, etc. These errors should be handled and mapped to `VCXFrameworkError`s wherever possible. The thought process here is that the framework should first indicate what action failed, with the causes determinable via the stack trace. For instance, an error of 'Serde deserialization failed' does not give a developer context as to what the issue is, while rather "Failed to deserialize inbound DIDComm message" does provide the necessary context.

> This architectural approach for error handling may need further adjustment as development on the framework progresses - @jameskebert
