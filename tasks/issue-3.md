# Issue #3, How to properly use it with kubectl logs -f ?

https://github.com/alexsavio/cor-cli/issues/3

**Labels:** -
**Milestone:** -

## Context

One of the examples in the README is this:

```bash
kubectl logs my-pod | cor --level warn
```

This works as expected.

However, if I add the `-f` or `--follow` flag to the `kubectl` command to keep watching the logs, it doesn't work. Nothing is printed.

I guess that's due to buffering, so I tried adding different things like `less -R`,  `stdbuf` and `script` with no success. Using `tee` to save the output of kubectl to a file while piping the logs `cor` I could confirm that it is receiving the logs but not printing anything.

I searched for a `cor` command line option related to buffering but it seems there is none.

What is the proper way to do it?

Thanks

Version: cor 2026.3.1
OS: Linux x86_64

## Acceptance criteria

- [ ] Streaming input (`kubectl logs -f`, `tail -f`, etc.) prints lines as they arrive without waiting for EOF or full buffer
- [ ] Output is flushed per line so downstream pagers / tee see each line immediately
- [ ] Existing batch behavior unchanged; tests still pass
- [ ] Add regression test covering line-streaming flush behavior

## Notes
