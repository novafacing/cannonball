# Performance

## Synthetic

Testing the send/receive encode/decode mechanism using Tokio, I saw a throughput of
about 3.58m events per second with a single producer and single consumer. That seems
good enough, but it is possible QEMU will saturate that speed, which is consumer-limited (I think).