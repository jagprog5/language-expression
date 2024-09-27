// #![no_std]

/// indicates the position in the input string in which something occurred
type InputOffset = usize;

/// indicates the number of tokens in the output that comprise something. always greater than 0
type OutputDelta = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Function<'a> {
    pub offset: InputOffset,
    pub name: &'a [u8],
    /// how many arguments does this function have
    pub num_args: usize,
    /// number of tokens to jump forward to be one past this function
    pub delta: OutputDelta,
    /// the number of tokens to jump forward to be at the end arg token for the first argument
    pub first_arg_delta: Option<OutputDelta>,
}

impl<'a> Default for Function<'a> {
    fn default() -> Self {
        Self {
            offset: 0,
            name: Default::default(),
            num_args: Default::default(),
            delta: Default::default(),
            first_arg_delta: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionArgEnd {
    pub offset: InputOffset,
    /// the number of tokens to jump forward to be at the end arg token for the next argument
    pub arg_delta: Option<OutputDelta>,
}

impl Default for FunctionArgEnd {
    fn default() -> Self {
        Self {
            offset: Default::default(),
            arg_delta: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Character {
    pub offset: InputOffset,
    pub val: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token<'a> {
    Invalid,
    Character(Character),
    Function(Function<'a>),
    FunctionArgEnd(FunctionArgEnd),
}

impl<'a> Default for Token<'a> {
    fn default() -> Self {
        Token::Invalid
    }
}

/// this function should be called in two passes. for the first pass, give None  
/// as the output arg, and the return value is the size of the output for the  
/// second pass. on err, gives the offending location and error reason.  
///
/// both the first AND second pass should be checked for err
///
/// stack is a scratch space used by this function, and must be the same len as input  
pub fn tokenize<'a>(
    input: &'a [u8],
    stack: &mut [usize],
    output: &mut Option<&mut [Token<'a>]>,
) -> Result<usize, (usize, &'static str)> {
    // the stack is actually two different stacks!
    // the first stack grows in the positive direction. it contains indices to the functions in output.
    // the second stack grows in the negative direction. it contains the indices to the beginning of args in the function.

    // they are needed since some fields need to be modified as the input is consumed.
    // for example, the Function num_args fields is incremented as args are found
    debug_assert!(input.len() == stack.len());

    let mut output_index = 0usize;
    let mut escaped = false;

    let mut function_stack_index = 0usize;
    let mut function_arg_begin_stack_index = stack.len();

    let mut function_name_begin: Option<usize> = None; // could be optimized with 0 as None

    fn send_output<'a>(
        token_to_send: Token<'a>,
        output_index: &mut usize,
        output: &mut Option<&mut [Token<'a>]>,
    ) {
        if let Some(o) = output {
            o[*output_index] = token_to_send;
        }
        *output_index += 1;
    }

    for (i, ch) in input.iter().enumerate() {
        match function_name_begin {
            None => {
                if escaped { 
                    match *ch {
                        b'{' | b'}' | b',' | b'\\' => {
                            // these four characters can be escaped.
                            send_output(
                                Token::Character(Character {
                                    offset: i - 1,
                                    val: *ch,
                                }),
                                &mut output_index,
                                output,
                            );
                        }
                        _ => {
                            // any other characters are not escaped and send
                            // things through literally
                            send_output(
                                Token::Character(Character {
                                    offset: i - 1,
                                    val: b'\\',
                                }),
                                &mut output_index,
                                output,
                            );
                            send_output(
                                Token::Character(Character {
                                    offset: i,
                                    val: *ch,
                                }),
                                &mut output_index,
                                output,
                            );
                        }
                    };
                    escaped = false;
                    continue;
                }

                if *ch == b'\\' {
                    escaped = true;
                    continue;
                }

                if *ch == b'{' {
                    function_name_begin = Some(i + 1);
                    continue;
                }

                if function_stack_index == 0 || (*ch != b',' && *ch != b'}') {
                    send_output(
                        Token::Character(Character {
                            offset: i,
                            val: *ch,
                        }),
                        &mut output_index,
                        output,
                    );
                    continue;
                }

                // increment num_args
                if let Some(o) = output {
                    match o[stack[function_stack_index - 1]] {
                        Token::Function(mut function) => {
                            function.num_args += 1;
                            o[stack[function_stack_index - 1]] = Token::Function(function);
                        }
                        _ => {
                            debug_assert!(false);
                            return Err((0, "internal error"));
                        }
                    }
                }

                // set size of argument
                if let Some(o) = output {
                    let index = stack[function_arg_begin_stack_index];
                    match o[index] {
                        Token::Function(mut function) => {
                            function.first_arg_delta = Some(output_index - index);
                            o[index] = Token::Function(function);
                        }
                        Token::FunctionArgEnd(mut function_arg_end) => {
                            function_arg_end.arg_delta = Some(output_index - index);
                            o[index] = Token::FunctionArgEnd(function_arg_end);
                        }
                        _ => {
                            debug_assert!(false);
                            return Err((0, "internal error"));
                        }
                    }
                }

                stack[function_arg_begin_stack_index] = output_index;

                send_output(
                    Token::FunctionArgEnd(FunctionArgEnd {
                        offset: i,
                        arg_delta: None,
                    }),
                    &mut output_index,
                    output,
                );

                if *ch == b'}' {
                    // since the function ended, pop it from the stack
                    function_stack_index -= 1;
                    function_arg_begin_stack_index += 1;

                    // set size of function
                    if let Some(o) = output {
                        let index = stack[function_stack_index];
                        match o[index] {
                            Token::Function(mut function) => {
                                function.delta = output_index - index;
                                o[index] = Token::Function(function);
                            }
                            _ => {
                                debug_assert!(false);
                                return Err((0, "internal error"));
                            }
                        }
                    }
                }
            }
            Some(v) => {
                // currently looking for the end of the function name
                if *ch == b',' || *ch == b'}' {
                    // end of function name found
                    let mut function = Function {
                        offset: v - 1,
                        name: &input[v..i],
                        num_args: 0,
                        delta: 0,
                        first_arg_delta: None,
                    };
                    function_name_begin = None;
                    if *ch == b',' {
                        stack[function_stack_index] = output_index;
                        function_stack_index += 1;

                        function_arg_begin_stack_index -= 1;
                        stack[function_arg_begin_stack_index] = output_index;

                        // equal is ok, since function_stack_index - 1 contains stack top,
                        // and function_arg_begin_stack_index contains stack top
                        debug_assert!(function_stack_index <= function_arg_begin_stack_index);

                        send_output(Token::Function(function), &mut output_index, output);
                        continue;
                    }
                    // b'}': not only was name completed, the whole function was completed
                    function.delta = 1;
                    send_output(Token::Function(function), &mut output_index, output);
                }
            }
        }
    }

    if let Some(v) = function_name_begin {
        return Err((v, "function name wasn't completed"))
    }

    if function_stack_index != 0 {
        match output {
            None => {
                // in the first, pass, not considered an error. need output to
                // correctly give error offset for this
                ()
            }
            Some(o) => {
                let input_index = match o[stack[function_stack_index - 1]] {
                    Token::Function(function) => function.offset,
                    _ => {
                        debug_assert!(false);
                        return Err((0, "internal error"));
                    }
                };
                return Err((input_index, "unclosed function"));
            }
        }
    }

    Ok(output_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        let input = b"abc";
        let mut stack = [0usize; 3];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 3;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Character(Character {
                offset: 0,
                val: b'a'
            })
        );
        assert_eq!(
            output[1],
            Token::Character(Character {
                offset: 1,
                val: b'b'
            })
        );
        assert_eq!(
            output[2],
            Token::Character(Character {
                offset: 2,
                val: b'c'
            })
        );
    }

    #[test]
    fn empty_function_name() {
        let input = b"{}";
        let mut stack = [0usize; 2];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 1;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Function(Function {
                offset: 0,
                name: &[],
                num_args: 0,
                delta: 1,
                first_arg_delta: None
            })
        );
    }

    #[test]
    fn function_no_args() {
        let input = b"{test}";
        let mut stack = [0usize; 6];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 1;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Function(Function {
                offset: 0,
                name: b"test",
                num_args: 0,
                delta: 1,
                first_arg_delta: None
            })
        );
    }

    #[test]
    fn function_one_arg() {
        let input = b"{test,abc}";
        let mut stack = [0usize; 10];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 5;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Function(Function {
                offset: 0,
                name: b"test",
                num_args: 1,
                delta: 5,
                first_arg_delta: Some(4),
            })
        );
        assert_eq!(
            output[1],
            Token::Character(Character {
                offset: 6,
                val: b'a'
            }),
        );
        assert_eq!(
            output[2],
            Token::Character(Character {
                offset: 7,
                val: b'b'
            }),
        );
        assert_eq!(
            output[3],
            Token::Character(Character {
                offset: 8,
                val: b'c'
            }),
        );
        assert_eq!(
            output[4],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 9,
                arg_delta: None
            })
        );
    }

    #[test]
    fn function_check_stack_size() {
        // do a series of checks to ensure that the stacks don't cross (they never can)
        let input_list = [b"{,{,{,{,{,", b"{{{{{{{{{{", b"{,,,,,,,,,"];
        for input in input_list.iter() {
            let mut stack = [0usize; 10];
            let _ = tokenize(*input, &mut stack, &mut None);
            // checks a debug assert, but does not care about the output
        }
    }

    #[test]
    fn function_two_arg() {
        let input = b"{n,1,2}";
        let mut stack = [0usize; 7];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 5;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Function(Function {
                offset: 0,
                name: b"n",
                num_args: 2,
                delta: 5,
                first_arg_delta: Some(2),
            })
        );
        assert_eq!(
            output[1],
            Token::Character(Character {
                offset: 3,
                val: b'1'
            }),
        );
        assert_eq!(
            output[2],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 4,
                arg_delta: Some(2)
            })
        );
        assert_eq!(
            output[3],
            Token::Character(Character {
                offset: 5,
                val: b'2'
            }),
        );
        assert_eq!(
            output[4],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 6,
                arg_delta: None
            })
        );
    }

    #[test]
    fn function_nested() {
        let input = b"{outer,{inner,a,b},1,2}z";
        let mut stack = [0usize; 24];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 12;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Function(Function {
                offset: 0,
                name: b"outer",
                num_args: 3,
                delta: 11,
                first_arg_delta: Some(6),
            })
        );
        assert_eq!(
            output[1],
            Token::Function(Function {
                offset: 7,
                name: b"inner",
                num_args: 2,
                delta: 5,
                first_arg_delta: Some(2),
            })
        );
        assert_eq!(
            output[2],
            Token::Character(Character {
                offset: 14,
                val: b'a'
            }),
        );
        assert_eq!(
            output[3],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 15,
                arg_delta: Some(2)
            })
        );
        assert_eq!(
            output[4],
            Token::Character(Character {
                offset: 16,
                val: b'b'
            }),
        );
        assert_eq!(
            output[5],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 17,
                arg_delta: None
            })
        );
        assert_eq!(
            output[6],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 18,
                arg_delta: Some(2)
            })
        );
        assert_eq!(
            output[7],
            Token::Character(Character {
                offset: 19,
                val: b'1'
            }),
        );
        assert_eq!(
            output[8],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 20,
                arg_delta: Some(2)
            })
        );
        assert_eq!(
            output[9],
            Token::Character(Character {
                offset: 21,
                val: b'2'
            }),
        );
        assert_eq!(
            output[10],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 22,
                arg_delta: None
            })
        );
        assert_eq!(
            output[11],
            Token::Character(Character {
                offset: 23,
                val: b'z'
            }),
        );
    }

    #[test]
    fn empty_arg() {
        let input = b"{,}";
        let mut stack = [0usize; 3];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 2;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Function(Function {
                offset: 0,
                name: &[],
                num_args: 1,
                delta: 2,
                first_arg_delta: Some(1),
            })
        );
        assert_eq!(
            output[1],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 2,
                arg_delta: None
            })
        );
    }

    #[test]
    fn escapes() {
        let input = b",\\{{a,\\,}";
        let mut stack = [0usize; 9];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 5;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Character(Character {
                offset: 0,
                val: b','
            }),
        );
        assert_eq!(
            output[1],
            Token::Character(Character {
                offset: 1,
                val: b'{'
            }),
        );
        assert_eq!(
            output[2],
            Token::Function(Function {
                offset: 3,
                name: b"a",
                num_args: 1,
                delta: 3,
                first_arg_delta: Some(2),
            })
        );
        assert_eq!(
            output[3],
            Token::Character(Character {
                offset: 6,
                val: b','
            }),
        );
        assert_eq!(
            output[4],
            Token::FunctionArgEnd(FunctionArgEnd {
                offset: 8,
                arg_delta: None
            })
        );
    }

    #[test]
    fn non_escapable_escapes() {
        // first two are passed through as is. third is escapable
        let input = b"\\a\\n\\{";
        let mut stack = [0usize; 6];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 5;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        assert_eq!(
            output[0],
            Token::Character(Character {
                offset: 0,
                val: b'\\'
            }),
        );
        assert_eq!(
            output[1],
            Token::Character(Character {
                offset: 1,
                val: b'a'
            }),
        );
        assert_eq!(
            output[2],
            Token::Character(Character {
                offset: 2,
                val: b'\\'
            }),
        );
        assert_eq!(
            output[3],
            Token::Character(Character {
                offset: 3,
                val: b'n'
            }),
        );
        assert_eq!(
            output[4],
            Token::Character(Character {
                offset: 4,
                val: b'{'
            }),
        );
    }

    #[test]
    fn unclosed_function_name() {
        let input = b"{hi";
        let mut stack = [0usize; 3];
        let cap = tokenize(input, &mut stack, &mut None);
        assert!(cap.is_err());
    }

    #[test]
    fn unclosed_function() {
        let input = b"{hi,ab";
        let mut stack = [0usize; 6];
        let cap = tokenize(input, &mut stack, &mut None);
        const OUTPUT_SIZE: usize = 3;
        assert_eq!(cap, Ok(OUTPUT_SIZE));
        let mut output = [Token::default(); OUTPUT_SIZE];
        let cap = tokenize(input, &mut stack, &mut Some(&mut output));
        assert!(cap.is_err());
    }
}
