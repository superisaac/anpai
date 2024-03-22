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
* run `cargo +nightly build` to build feel interpreter feel
* run `cargo +nightly test` to run testing

## examples
```shell

% ./target/debug/anpai feel -c '"hello " + "world"'
"hello world"

% ./target/debug/anpai feel -c '(function(a, b) a + b)(5, 8)'
13

# dump AST tree instead of evaluating the script
% ./target/debug/anpai feel -c 'bind("a", 5); if a > 3 then "larger" else "smaller"' --ast
(expr-list (call bind ["a", 5]) (if (> a 3) "larger" "smaller"))

% ./target/debug/anpai feel -c 'some x in [3, 4, 8, 9] satisfies x % 2 = 0'
4

% ./target/debug/anpai feel -c 'every x in [3, 4, 8, 9] satisfies x % 2 = 0'
[4, 8]
```

for more examples please refer to testing
