# Party-API

This is a REST API backend specifically designed to streamline the visitor registration process for
[demoparties](https://en.wikipedia.org/wiki/Demoscene#Parties). To maintain simplicity, all data is persisted in an
SQLite database.

## Running

The following environment variables are used for configuration:

| Variable    | Description                  | Default value  |
|-------------|------------------------------|----------------|
| SQLITE_DB   | Path to SQLite database file | data.db        |
| LISTEN_ADDR | IP and port to listen on     | 127.0.0.1:3000 |

### Sample Docker Compose

Create a `docker-compose.yml` file with the following content.

```yml
services:
  party-api:
    image: ghcr.io/memoryleek/party-api:latest
    ports:
      - 3000:3000
    volumes:
      - ./party-api-data:/data
    environment:
      - SQLITE_DB=/data/party-api.db
```

Then run `docker-compose up -d` to start it. The SQLite database will be stored outside the container as
`party-api-data/party-api.db`.

## Example requests

### Fetching registered visitors

```sh
curl -i -H 'Accept: application/json' http://localhost:3000/visitors
```

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 73
date: Sat, 10 Jun 2023 19:16:20 GMT

[
  {
    "nick": "Lorem",
    "group": null
  },
  {
    "nick": "Ipsum Dolor",
    "group": "Sit Amet"
  }
]
```

### Registering as a visitor

Note that the fields `email` and `extra` are not shown in the public `GET /visitors` listing, but are intended only
for the party organizers.

```sh
curl -i -H 'Content-Type: application/json' \
     -X POST \
     -d '{"nick":"Lorem","group":"Ipsum","email":"lorem@example.com","extra":"Allergic to metaballs"}' \
     http://localhost:3000/register
```

```
HTTP/1.1 201 Created
content-length: 0
date: Sat, 10 Jun 2023 19:17:23 GMT
```