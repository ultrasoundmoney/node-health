# Node Health

Small service intended to check the readiness of a geth / lighthouse node pair. Exposes this readiness state over an API endpoint so kubernetes can be aware of it. Intended to run as the third container in a geth, lighthouse pod.
