# Persisted Query to REST Endpoint

This project enables teams to convert [Apollo's Persisted Queries](https://www.apollographql.com/docs/graphos/operations/persisted-queries/) hashes into a defined endpoint, enabling teams to provide REST endpoints to teams that require them while still using GraphQL.

## ⚠️ Disclaimer ⚠️

**The code in this repository is experimental and has been provided for reference purposes only. Community feedback is welcome but this project may not be supported in the same way that repositories in the official [Apollo GraphQL GitHub organization](https://github.com/apollographql) are. If you need help you can file an issue on this repository, [contact Apollo](https://www.apollographql.com/contact-sales) to talk to an expert, or create a ticket directly in Apollo Studio.**

## Features

- **Mapping**: Allows you to convert a given persisted query (PQ) hash into a defined REST endpoint using YAML
- **Argument location support**: This project allows you to pass GraphQL arguments using different names and locations, including the body, query parameters, and/or path arguments
- **Status code propagation**: This will propagate status codes over the default `2XX` if your GraphQL endpoint returns a different one than the norm. If it returns a `2XX` status code with errors, `persisted-query-to-rest` will return a `500` to properly denote errors
- **Header propagation**: `persisted-query-to-rest` will propagate any returned headers with exceptions for standard ones to transport the data back to the client, such as `content-encoding` and `content-length`. Other headers, such as `cache-control`, will be passed back as-is

## Getting Started

### Downloading a release
You can run this project as a [Docker image](https://github.com/apollosolutions/persisted-query-to-rest/pkgs/container/persisted-query-to-rest) or manually running the pre-compiled [release binary](https://github.com/apollosolutions/persisted-query-to-rest/releases).

Here's how to use the pre-built Docker image, mounting a local `config.yml` to a container and using it as a configuration:
```
docker run -p 8080:8080 --mount "type=bind,source=./config.yml,target=/app/config.yml" ghcr.io/apollosolutions/persisted-query-to-rest:latest --config /app/config.yml
```

## Usage

There is one flag for `persisted-query-to-rest`:

* `--config`, which specifies the config location

## Configuration

See [`example_config.yaml`](./example_config.yaml) for a complete example of a configuration.

### Generate a configuration schema
To generate a YAML schema file you can run the binary with the command `config-schema`. This file can then be configured in your config YAML file to provide autocomplete and validation of the YAML file right in your IDE

```shell
# forces the application to print out a JSON schema for YAML validation
./persisted-query-to-rest config-schema > config-schema.json
```
OR
```shell
docker run ghcr.io/apollosolutions/persisted-query-to-rest:latest config-schema > config-schema.json
```

* See [example_config.yaml](./example_config.yaml) to see an example of setting it up
* We use [Red Hat's YAML extension for VSCode](https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml) to validate

### Common

This configuration contains the general configuration settings for `persisted-query-to-rest`. 

* **path_prefix**: The path that prefixes all endpoints; for example the default of `/api/v1` would lead to an endpoint like `http://localhost:4000/api/v1/users`
* **listen**: The address it should listen on
* **graphql_endpoint**: The GraphQL server that should be hit for the operations
* **logging**: The logging configuration for the endpoint. See [Logging](#logging) below for configuration options

#### Logging

* **level**: The level at which the service should log. By default it is set to `info`, but can be set to higher/lower values as needed

### Endpoints

The endpoints lists the endpoint mappings for `persisted-query-to-rest` to serve. An endpoint represents a REST endpoint mapped to a given PQ hash/ID. 

* **path**: The path that the endpoint should be exposed on. If wanting to use path arguments, the format is `:<variable_name>`, for example `/user/:id` has an argument name of `id`
* **method**: The method that the endpoint should accept; acceptable values are `GET`, `POST`, `PATCH`, `DELETE`, and `PUT`
* **pq_id**: The persisted query ID that the endpoint should use
* **query_params**: The list of  query parameters that the endpoint should use for variables. For more information on argument configuration, see [Parameters](#parameters) below
* **path_arguments**: The list of  path arguments that the endpoint should use for variables. For more information on argument configuration, see [Parameters](#parameters) below
* **body_params**: The list of body parameters that the endpoint should use for variables. For more information on argument configuration, see [Parameters](#parameters) below

#### Parameters

* **from**: The parameter name that the user will use; e.g. `id` in `/user/:id` or /user/?id=1234
* **to**: If the operation variable uses a different name, this is the name the variable should be renamed to
* **required**: Whether the parameter is required or not; by default it is false
* **kind**: he kind of parameter that is expected if it is not a string

The `kind` argument accepts the various kinds of JSON scalars: `int`, `string` (default), `float`, `object`, `array`, or `boolean`. When setting the `kind`, make sure it matches the expected input from the consumer. 

If the type is an `object`, any further downstream properties of that object will be parsed and sent exactly as-is, as will `array`. 

## Known Limitations

- Array arguments in query parameters are not supported as multi-value entries (e.g. `?id=1&id=2`), but if needed, passing as a raw array string is supported (e.g. `?ids=[1,2,3,4]`)
- When a parameter `kind` is set to `object`, there is no way to address specific sub-attributes for variables, only passing it as an object directly
