# Party-API

This is a REST API backend specifically designed to streamline the visitor registration process for
[demoparties](https://en.wikipedia.org/wiki/Demoscene#Parties). To maintain simplicity, all data is persisted in an
SQLite database.

## Examples

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
curl -i -H 'Content-Type: application/json' -X POST -d '{"nick":"Lorem","group":"Ipsum","email":"lorem@example.com","extra":"Allergic to metaballs"}' http://localhost:3000/register
```

```
HTTP/1.1 201 Created
content-length: 0
date: Sat, 10 Jun 2023 19:17:23 GMT
```