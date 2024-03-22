# CS128H Project

## Group Name

Biting off more than you can chew incorporated

## Group Members

- Jenna Fligor (`jflig2`)
- Caden Raquel (`craquel2`)

## Product introduction

Our project is a POSIX-gazing shell language. What we mean by that is we're not trying to make our shell POSIX compliant, but rather we're modeling our functionality after what POSIX defines while refining the syntax to be a little cleaner. An additional motivation of forgoing POSIX compliance is to give us some flexibility in implementation, a shell is a very complex piece of software and we don't expect to have a 100% complete product by the end of the class, we're hoping to complete everything under the "basic shell functionality" goals, but the other goals are largely aspirational.

- [ ] basic shell functionality
  - [ ] run commands
  - [ ] implement shell builtins like `cd`, `if`, `while`, and `for`
  - [ ] manipulate environment variables
  - [ ] environment variable interpolation
  - [ ] direct command output to file
- [ ] advanced shell functionality
  - [ ] shell interpolation
  - [ ] pipes
  - [ ] configurable prompt
  - [ ] script support

## Technical Overview

As previously mentioned a shell is a very complicated piece of software, so we're gonna try to keep it as simple as possible, and split it into four major components, the frontend, the parser, the evaluator, and the process manager.

```
┌──────────────────────────┐ ┌───────────────────┐
│ Stdin, Stdout, & Stderr  │ │ Other Files/Pipes │
└──┬───────────────────┬───┘ └────┬──────────────┘
   │ ▲                 │  ▲    ▲  │
   │ │                 │  │    │  │
   ▼ │                 │  │    │  ▼
┌────┴─────┐           │  │ ┌──┴────────┐
│          │           │  └─┤ Process   │  shell interpolations
│ Frontend │           │    │ Manager   │  (std::String)
│          │           └───►│           ├───────┐
└──┬───────┘                └───────────┘       │
   │                           ▲                │
   │ command string            │ command entry  │
   │ (std::String)             │ (Custom Type)  │
   ▼                           │                │
┌──────────┐                ┌──┴────────┐       │
│          │                │           │◄──────┘
│ Parser   ├───────────────►│ Evaluator │
│          │   abstract     │           │
└──────────┘   syntax       └───────────┘
               tree
               (Custom Type)
```

### Frontend

This part of the program will probably be the simplest, it's purpose it to interact with the user and convert what it types into a string to be passed off to the parser. It will facilitate interactions with the user, print the prompt, read lines from the user, and parse trailing backslashes for line continuation.

### Parser

This will probably be the hardest part of the program, we'll probably use something like the [`pest`](https://pest.rs/) crate to define formal parsing expression grammar, which will make the parser relatively painless, but this part will also need to convert the string-based tree produced by the parser into a more compact abstract syntax tree (AST) before passing it off to the evaluator.

### Evaluator

This part of the program will evaluate the AST an create command entry, which specify a command, and it's arguments with interpolations substituted to the process manager, it will dispatch shell interpolations to the process manager and substitute environment variables to "flatten" a command down to string literals before dispatching it to the process manager

### Process Manager

This part of the program will perform builtin shell operations as well as instantiating child processes and directing their stdins and stdouts to the appropriate files/pipes where appropriate.

### Timeline

given that we both already know rust fairly well the tentative timeline is 1-2 weeks for the core functionality of the frontend, then 2-3 weeks for each aditional component, with another 2-3 weeks to debug inter-component comunication errors. at that point we can work on more advanced functionality, for example the while we'll write the parser to support shell interpolation we may not implement it in the evaluator until the basic "run a simple command" functionality of the whole project is working.

## Possible Challenges

The single biggest challenge of this project is the overall difficulty of the project, it's a complicated piece of software with a lot of moving parts, so it's gonna be a challenge and need lots of debugging.

## References

This project is slightly inspired by a [simpler project Jenna did several months ago](https://github.com/Ex-32/jnk); a REPL that does basic arithmetic operations with bigints
