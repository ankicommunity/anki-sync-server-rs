# Reverse Proxy Setup

How to setup a reverse proxy using nginx.

Install and expose the sync server to the reverse proxy server at adress and port `SYNC_SERVER_ADDR:SYNC_SERVER_PORT` (loopback `127.0.0.1` or firewalled traffic inside controlled network).

Inside the server directive of the host you want to use for anki add the following `location /` block:

```
server {
  listen 443 ssl;
  # Increase nginx version identification difficulty
  server_tokens off;
  ...
  # Increase body size
  client_max_body_size 512M;
  # Pass traffic to sync server²
  location / {
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

    # Set headers for more security
    #add_header X-Content-Type-Options nosniff;
    #add_header X-Frame-Options "SAMEORIGIN";
    #add_header X-XSS-Protection "1; mode=block";
    #add_header X-Robots-Tag none;
    #add_header X-Download-Options noopen;
    #add_header X-Permitted-Cross-Domain-Policies none;
    #add_header Referrer-Policy 'strict-origin';
    #add_header Front-End-Https on;
    proxy_pass http://SYNC_SERVER_ADDR:SYNC_SERVER_PORT;
  }
```
