#!/bin/bash
# Start Nginx in background
nginx -g "daemon on;"
# Start overseer server in foreground
/app/overseer server
