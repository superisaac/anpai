# anpai project
The project of anpai(namely 安排 in Chinese) is a suite of BPMN and
DMN spec implementations in rust language.

## feel-lang
The interpreter of the FEEL language(Friendly Enough Expression
Language) in rust, FEEL is broadly used in DMN and BPMN to provide rule
engine and script support, the FEEL module can be included into
other rust projects or used as command line executable as FEEL
interpreter.

## build
* run `cargo build` to build feel interpreter feel
* run `cargo test` to run testing

## examples
### FEE interpreter
```shell

% ./target/debug/anpai feel -c '"hello " + "world"'
"hello world"

% ./target/debug/anpai feel -c '(function(a, b) a + b)(5, 8)'
13

% ./target/debug/anpai feel -c '>8,<4' --vars '{"?": 6}' --top unary-tests
false

# dump AST tree instead of evaluating the script
% ./target/debug/anpai feel -c 'if a > 3 then "larger" else "smaller"' --vars '{a: 5}' --ast
(if (> a 3) "larger" "smaller")

% ./target/debug/anpai feel -c 'some x in [3, 4, 8, 9] satisfies x % 2 = 0'
4

% ./target/debug/anpai feel -c 'every x in [3, 4, 8, 9] satisfies x % 2 = 0'
[4, 8]
```

### DMN evaluator
```shell
# evaluate dmn files, given context vars
% ./target/debug/anpai dmn examples/dmn/simpledish.dmn --vars '{season: "Summer", guestCount: 10, guestsWithChildren: true}'
{"Beverages":"Apple Juice"}
```

for more examples please refer to testing
