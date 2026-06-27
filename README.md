# Party-API

This is a REST API backend specifically designed to streamline the visitor registration process for
[demoparties](https://en.wikipedia.org/wiki/Demoscene#Parties). To maintain simplicity, all data is persisted in an
SQLite database.

## Running

The following environment variables are used for configuration:

| Variable    | Description                         | Default value  |
|-------------|-------------------------------------|----------------|
| API_KEY     | Key protecting the /admin endpoints |                |
| CORS_ORIGIN | CORS preflight URL restriction      | *              |
| SQLITE_DB   | Path to SQLite database file        | data.db        |
| LISTEN_ADDR | IP and port to listen on            | 127.0.0.1:3000 |

### Sample Docker Compose

Create a `docker-compose.yml` file with the following content, replacing `myapikey` with your own key.

```yml
services:
  party-api:
    image: ghcr.io/memoryleek/party-api:latest
    ports:
      - 3000:3000
    volumes:
      - ./party-api-data:/data
    environment:
      - API_KEY=myapikey
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
    "id": 1,
    "nick": "Lorem",
    "group": null
  },
  {
    "id": 2,
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

### Fetching full visitor data

This is only available for organizers, authorized by API_KEY.

```sh
curl -i -H 'Accept: application/json' \
     -H 'Authorization: Bearer myapikey' \
     http://localhost:3000/admin/visitors
```

```
HTTP/1.1 200 OK
content-type: application/json
content-length: 277
date: Tue, 04 Jul 2023 18:28:11 GMT

[
  {
    "id":1,
    "created_at":"2023-07-04T18:26:51.724571400Z",
    "ip":"127.0.0.1:49580",
    "nick":"Lorem",
    "group":null,
    "email":null,
    "extra":null
  },
  {
    "id":2,
    "created_at":"2023-07-04T18:26:56.288133200Z",
    "ip":"127.0.0.1:49582",
    "nick":"Ipsum Dolor",
    "group":"Sit Amet",
    "email":null,
    "extra":null
  }
]
```

### Deleting a visitor

This is only available for organizers, authorized by API_KEY.

```sh
curl -i -H 'Accept: application/json' \
     -H 'Authorization: Bearer myapikey' \
     -X DELETE \
     http://localhost:3000/admin/visitors/1
```

```
HTTP/1.1 204 No Content
content-length: 0
date: Tue, 04 Jul 2023 18:30:56 GMT
```
