# Lua Design

How lua works

## Process Overview

### Parsing

1. Input: character stream
  - Do lexical analysis
2. Intermediate: tokenstream
  - Do syntax analysis
3. Output: bytecode

The bytecode is then run by a lua runtime
 

