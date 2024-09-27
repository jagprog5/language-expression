# Language Expression

This is a no dependency and `no_std` lib for tokenizing an expression that supports characters and functions.

- linear in space and time with the size of the input
- simple (< 300 LOC)
- tokens are informative, and allow for visiting arguments in an efficient manner

## The Language

### Quick Examples

- `hello {function_name,arg0,arg1,arg2}`
- `{outer,{inner,a,b},1,2}z`

### Specifics

The only _special_ characters are:

 - `\` escape character
 - `{` begin function
 - `,` argument separator
 - `}` end function

#### Functions

A function has a name and a number of arguments: `{name,arg,arg}`.  
A function with a zero length name and no arguments looks like `{}`.  
A function with no arguments looks like `{name}`.

Functions can be _nested_, meaning a function's argument can also be a function, ad infinitum.

A function name is terminated by either `}` or `,` but can contain any other character. While a function name is being scanned, everything else (including `\`) is treated literally.

#### Escapes

The escape character escapes the functionality of the following character. For example, if `{` would usually start a function, then `\{` will instead emit a character literal.

Only the _special_ characters can be escaped. If something else is escaped, for example `\n`, then two character literals are emitted: `\`, `n`.

When not within a function, it is not necessary to escape `,` or `}`. It's redundant, but allowed.

## Tokens

Let's looks at a specific example and its token representation:  

Input: `{outer,{inner,ab,c},1,2}z`

Output:

```txt
0  FUNCTION
       name: "outer",
       number args: 3
       delta: 12
       first arg delta: 7
1      |   FUNCTION
       |       name: "inner"
       |       number args: 2,
       |       delta: 6
       |       first arg delta: 3
2      |   CHARACTER 'a'
3      |   CHARACTER 'b'
4      |   END_ARG (delta: 2)
5      |   CHARACTER 'b'
6      |   END_ARG (delta: None)
7  END_ARG (delta: 2)
8  CHARACTER '1'
9  END_ARG (delta: 2)
10 CHARACTER '2'
11 END_ARG (delta: None)
12 CHARACTER 'z'
```

Every token has an `offset` which indicates where in the input string it came from. For example the last token, which is for `z`, came from the character at offset 24 of the input string.

Functions and arguments have a `delta`. This is the number of tokens to skip forward to reach the next element.
 - Moving forward by a function delta will point to one past the function. In the example, the "outer" function's delta points to the character 'z'.
 - Argument deltas form a singly linked list pointing to the END_ARG marker for the next argument.

Tokens store information in a redundant way to be convenient. For example, a `FUNCTION` token has a `num_args` member. The number of arguments can also be known by walking the argument deltas until the end is reached.

## Diagnostics

Diagnostic information is given on error, in the form of an offending offset and reason.

```txt
{hi
^ function name wasn't completed
{hi,ab
^ unclosed function
```
