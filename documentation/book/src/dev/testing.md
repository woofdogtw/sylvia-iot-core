# Writing Tests

Sylvia-IoT adopts the BDD (Behavior-Driven Development) approach for writing integration tests, and
the chosen testing framework [**laboratory**](https://enokson.github.io/laboratory/) is based on
[**Mocha**](https://mochajs.org/).

This section will focus on the principles and techniques for writing tests for libs, models, and
routes.

## TestState

The `TestState` structure is used as a parameter for `SpecContext()`. It keeps track of several
variables:

- Variables that exist for a long time and only need to be initialized once or very few times, such
  as `runtime` and `mongodb`.
- Resources that need to be released in `after`. Since test cases may exit abruptly, it is essential
  to release resources in `after`.

## libs

- Simple functions can be tested directly for their inputs and outputs.
- Before testing, ensure to start the necessary infrastructure, such as RabbitMQ, EMQX, etc.
- For more complex scenarios that require services to be set up, you can create the services (e.g.,
  queue connections) in `before` and release them in `after`.

## models

- Before testing, make sure to start MongoDB, Redis, and other databases.
- The test order should be R, C, U, D.
    - **R**: Use `mongodb`, `sqlx`, or other native packages to create a test dataset, then test the
      results of model's get, count, and list functions.
    - **C**: Use model's add, upsert, or other functions to create data and validate its correctness
      using get.
    - **U**: Use model's add, upsert, or other functions to create a test dataset, then use update
      to modify the data, and finally validate the result using get.
    - **D**: Use model's add, upsert, or other functions to create a test dataset, then use delete
      to delete the data, and finally validate the result using get.
    - Test **R** functionalities first to enable writing C, U, D test cases using unified code and
      determine if the same logic results in the same outcome for each database engine. When
      introducing new engines, you can write minimal test code for testing.
- Use native packages for deleting in `after`. This is because you cannot guarantee that D-related
  functionalities are correctly implemented and tested before testing.

## routes

- Although you can use Actix Web's `test::init_service()` as a virtual service, services required by
  middleware or API bridges need to be started using threads.
- You can use model trait interfaces for initializing test datasets and data validation after API
  requests.
- You can use model delete to delete test data in `after`.
