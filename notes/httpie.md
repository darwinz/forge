---
description: HTTPie CLI examples
category: General
---
https://httpie.io/docs/cli/examples

  http PUT pie.dev/put X-API-Token:123 name=John
  http -f POST pie.dev/post hello=World
  http -v pie.dev/get

  # Use https
  https example.org

  # Build and print a request without sending it
  http --offline pie.dev/post hello=offline

  # With authentication
  http -a USERNAME POST https://api.github.com/repos/httpie/httpie/issues/83/comments body='HTTPie is awesome!'

  # Upload a file
  http pie.dev/post < files/data.json

  # Download a file
  http pie.dev/image/png > image.png

  # Using named session to persist between requests
  http --session=logged-in -a username:password pie.dev/get API-Key:123
  http --session=logged-in pie.dev/headers

  # Use query string params
  http https://api.github.com/search/repositories q==httpie per_page==1
