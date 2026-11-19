use crate::code::{CodeLanguage, generate_code, parse_code};
use crate::structs::VplState;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum CodeSyncStatus {
    #[default]
    NotLoaded,
    Synced,
    SyntaxError(String),
}

#[derive(Default)]
pub(crate) struct CodeSync {
    source_id: Option<usize>,
    language: CodeLanguage,
    code: String,
    generated: String,
    status: CodeSyncStatus,
    write_error: Option<String>,
}

impl VplState {
    pub fn load_code(&mut self, source_id: usize, code: &str) {
        let sync = &mut self.sync;
        sync.source_id = Some(source_id);
        sync.language = self.language;
        sync.code = code.to_string();
        sync.write_error = None;

        match parse_code(code, self.language) {
            Ok(parsed) => {
                sync.generated = generate_code(&parsed.statements, self.language);
                sync.status = CodeSyncStatus::Synced;
                self.blocks = parsed.statements;
            }
            Err(message) => {
                sync.generated.clear();
                sync.status = CodeSyncStatus::SyntaxError(message);
                self.blocks.clear();
            }
        }
    }

    pub fn sync_status(&self) -> &CodeSyncStatus {
        &self.sync.status
    }

    pub fn write_error(&self) -> Option<&str> {
        self.sync.write_error.as_deref()
    }

    pub(crate) fn sync_from_code(&mut self, source_id: usize, code: &str) {
        if self.sync.source_id != Some(source_id)
            || self.sync.language != self.language
            || self.sync.code != code
        {
            self.load_code(source_id, code);
        }
    }

    pub(crate) fn write_back(&mut self, code: &mut String) -> bool {
        if self.sync.status != CodeSyncStatus::Synced {
            return false;
        }

        let generated = generate_code(&self.blocks, self.language);
        if generated == self.sync.generated {
            self.sync.write_error = None;
            return false;
        }

        let check = match parse_code(&generated, self.language) {
            Ok(parsed) if parsed.statements == self.blocks => Ok(()),
            Ok(_) => Err("The generated code doesn't match the blocks.".to_string()),
            Err(message) => Err(message),
        };
        if let Err(message) = check {
            self.sync.write_error = Some(message);
            return false;
        }
        self.sync.write_error = None;

        *code = generated.clone();
        self.sync.code = generated.clone();
        self.sync.generated = generated;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::{CodeLanguage, statements_to_string};
    use keystone_lang::parse;
    use keystone_lang::{Callee, Direction, Expr, Op, Statement, UnaryOp};

    fn sample_blocks() -> Vec<Statement> {
        vec![
            Statement::Move(Expr::Direction(Direction::Forward)),
            Statement::Turn(Expr::Direction(Direction::Left)),
            Statement::Dig(Expr::Direction(Direction::Up)),
            Statement::Place(Expr::Direction(Direction::Down)),
            Statement::Print(Expr::String("hello".to_string())),
            Statement::Sleep(Expr::Float(0.25)),
            Statement::Let("x".to_string(), Expr::Uint(1)),
            Statement::Send(Expr::String("signal".to_string())),
            Statement::Receive(Expr::String("signal".to_string())),
            Statement::If(
                Expr::Call {
                    callee: Callee::IsTouched,
                    args: vec![],
                },
                vec![Statement::Loop(
                    Expr::Uint(3),
                    vec![Statement::Move(Expr::Direction(Direction::Back))],
                )],
                None,
            ),
            Statement::While(
                Expr::Binary {
                    op: Op::And,
                    lhs: Box::new(Expr::Unary {
                        op: UnaryOp::Not,
                        exp: Box::new(Expr::Boolean(false)),
                    }),
                    rhs: Box::new(Expr::Binary {
                        op: Op::Lt,
                        lhs: Box::new(Expr::Var("x".to_string())),
                        rhs: Box::new(Expr::Call {
                            callee: Callee::Rand,
                            args: vec![Box::new(Expr::Uint(1)), Box::new(Expr::Uint(10))],
                        }),
                    }),
                },
                vec![],
            ),
            Statement::If(
                Expr::Call {
                    callee: Callee::IsEmpty,
                    args: vec![Box::new(Expr::Direction(Direction::Right))],
                },
                vec![Statement::Let(
                    "x".to_string(),
                    Expr::Binary {
                        op: Op::Add,
                        lhs: Box::new(Expr::Var("x".to_string())),
                        rhs: Box::new(Expr::Uint(1)),
                    },
                )],
                None,
            ),
        ]
    }

    fn print(text: &str) -> Statement {
        Statement::Print(Expr::String(text.to_string()))
    }

    fn if_else(then_body: Vec<Statement>, else_body: Option<Vec<Statement>>) -> Statement {
        Statement::If(Expr::Boolean(true), then_body, else_body)
    }

    #[test]
    fn else_branches_round_trip_through_code() {
        let nested_loop = Statement::Loop(Expr::Uint(2), vec![if_else(vec![], Some(vec![]))]);
        for blocks in [
            vec![if_else(vec![print("a")], Some(vec![print("b")]))],
            vec![if_else(vec![], Some(vec![]))],
            vec![if_else(
                vec![print("a")],
                Some(vec![if_else(
                    vec![print("b")],
                    Some(vec![if_else(vec![print("c")], Some(vec![print("d")]))]),
                )]),
            )],
            vec![if_else(
                vec![print("a")],
                Some(vec![if_else(vec![print("b")], None)]),
            )],
            // An `if` that shares the else-branch with other blocks is not an `else if`.
            vec![if_else(
                vec![],
                Some(vec![if_else(vec![print("b")], None), print("c")]),
            )],
            vec![nested_loop],
        ] {
            let code = statements_to_string(&blocks, 0);
            assert_eq!(parse(&code).unwrap(), blocks, "generated code:\n{code}");
        }
    }

    #[test]
    fn a_lone_if_in_the_else_branch_is_written_as_else_if() {
        let blocks = vec![if_else(
            vec![print("a")],
            Some(vec![if_else(vec![print("b")], Some(vec![print("c")]))]),
        )];

        assert_eq!(
            statements_to_string(&blocks, 0),
            "if true\n  print \"a\"\nelse if true\n  print \"b\"\nelse\n  print \"c\"\nend\n"
        );
    }

    fn rhai_state(code: &str) -> VplState {
        let mut state = VplState {
            language: CodeLanguage::Rhai,
            ..VplState::default()
        };
        state.load_code(0, code);
        state
    }

    #[test]
    fn rhai_code_loads_as_blocks_and_edits_write_rhai_back() {
        let mut code = "// keep left\nmove(\"left\");".to_string();
        let mut state = rhai_state(&code);
        assert_eq!(state.sync_status(), &CodeSyncStatus::Synced);
        assert_eq!(
            state.blocks,
            [Statement::Move(Expr::Direction(Direction::Left))]
        );

        assert!(!state.write_back(&mut code));
        assert_eq!(code, "// keep left\nmove(\"left\");");

        state
            .blocks
            .push(Statement::Let("x".to_string(), Expr::Uint(1)));
        assert!(state.write_back(&mut code));
        assert_eq!(code, "move(\"left\");\nlet x = 1;\n");
    }

    #[test]
    fn rhai_the_blocks_cannot_show_is_reported_and_left_alone() {
        let mut code = "fn go() { move(\"up\"); }".to_string();
        let mut state = rhai_state(&code);
        assert!(matches!(
            state.sync_status(),
            CodeSyncStatus::SyntaxError(message) if message.contains("fn")
        ));
        assert!(!state.write_back(&mut code));
        assert_eq!(code, "fn go() { move(\"up\"); }");
    }

    #[test]
    fn rhai_blocks_that_rhai_cannot_express_are_not_written_back() {
        let mut code = String::new();
        let mut state = rhai_state(&code);
        state
            .blocks
            .push(Statement::Turn(Expr::Direction(Direction::Left)));
        assert!(!state.write_back(&mut code));
        assert!(state.write_error().is_some());
        assert_eq!(code, "");
    }

    #[test]
    fn changing_the_language_reloads_the_same_text() {
        let code = "move(\"up\");";
        let mut state = VplState::default();
        state.sync_from_code(0, code);
        assert!(matches!(
            state.sync_status(),
            CodeSyncStatus::SyntaxError(_)
        ));

        state.language = CodeLanguage::Rhai;
        state.sync_from_code(0, code);
        assert_eq!(state.sync_status(), &CodeSyncStatus::Synced);
    }

    #[test]
    fn inspection_tells_whether_code_can_switch_to_blocks() {
        use crate::code::{CodeInspection, inspect_code};

        assert_eq!(
            inspect_code("move(\"up\"); // go", CodeLanguage::Rhai),
            CodeInspection::Representable { has_comments: true }
        );
        assert_eq!(
            inspect_code("", CodeLanguage::Rhai),
            CodeInspection::Representable {
                has_comments: false
            }
        );
        assert!(matches!(
            inspect_code("loop { }", CodeLanguage::Rhai),
            CodeInspection::Unrepresentable(_)
        ));
        assert_eq!(
            inspect_code("move up", CodeLanguage::Keystone),
            CodeInspection::Representable {
                has_comments: false
            }
        );
        assert!(matches!(
            inspect_code("move", CodeLanguage::Keystone),
            CodeInspection::Unrepresentable(_)
        ));
    }

    #[test]
    fn editing_an_else_branch_writes_code_back() {
        let mut state = VplState::default();
        let mut code = "if true\n  move left\nelse\nend\n".to_string();
        state.load_code(0, &code);
        let Statement::If(_, _, Some(else_body)) = &mut state.blocks[0] else {
            panic!("expected an if-else block, got {:?}", state.blocks);
        };
        else_body.push(Statement::Move(Expr::Direction(Direction::Right)));

        assert!(state.write_back(&mut code));
        assert_eq!(code, "if true\n  move left\nelse\n  move right\nend\n");
    }

    #[test]
    fn blocks_round_trip_through_code() {
        let blocks = sample_blocks();
        let code = statements_to_string(&blocks, 0);
        assert_eq!(parse(&code).unwrap(), blocks, "generated code:\n{code}");
    }

    #[test]
    fn oversized_uint_literal_is_a_syntax_error() {
        let mut state = VplState::default();
        state.load_code(0, "move 99999999999");

        assert!(matches!(
            state.sync_status(),
            CodeSyncStatus::SyntaxError(_)
        ));
    }

    #[test]
    fn loading_code_does_not_rewrite_it() {
        let mut state = VplState::default();
        let mut code = "move   forward\n\n\nturn left".to_string();
        state.sync_from_code(0, &code);

        assert_eq!(state.sync_status(), &CodeSyncStatus::Synced);
        assert_eq!(state.blocks.len(), 2);
        assert!(!state.write_back(&mut code));
        assert_eq!(code, "move   forward\n\n\nturn left");
    }

    #[test]
    fn editing_blocks_writes_code_back() {
        let mut state = VplState::default();
        let mut code = "move forward".to_string();
        state.sync_from_code(0, &code);

        state
            .blocks
            .push(Statement::Turn(Expr::Direction(Direction::Left)));
        assert!(state.write_back(&mut code));
        assert_eq!(parse(&code).unwrap(), state.blocks);

        // 書き戻した後は、同じコードで読み込み直さない
        state
            .blocks
            .push(Statement::Dig(Expr::Direction(Direction::Up)));
        state.sync_from_code(0, &code);
        assert_eq!(state.blocks.len(), 3);
    }

    #[test]
    fn switching_source_reloads_blocks() {
        let mut state = VplState::default();
        state.sync_from_code(0, "move forward");
        state.sync_from_code(1, "turn left\nturn right");

        assert_eq!(
            state.blocks,
            vec![
                Statement::Turn(Expr::Direction(Direction::Left)),
                Statement::Turn(Expr::Direction(Direction::Right)),
            ]
        );
    }

    #[test]
    fn broken_blocks_are_not_written_back() {
        let mut state = VplState::default();
        let mut code = "print \"hello\"".to_string();
        state.sync_from_code(0, &code);

        state.blocks[0] = Statement::Print(Expr::String("hello \"world\"".to_string()));
        assert!(!state.write_back(&mut code));
        assert_eq!(code, "print \"hello\"");
        assert!(state.write_error().is_some());

        state.blocks[0] = Statement::Print(Expr::String("hello world".to_string()));
        assert!(state.write_back(&mut code));
        assert_eq!(code, "print \"hello world\"\n");
        assert!(state.write_error().is_none());
    }

    #[test]
    fn blocks_that_read_back_differently_are_not_written_back() {
        let mut state = VplState::default();
        let mut code = "print x".to_string();
        state.sync_from_code(0, &code);

        state.blocks[0] = Statement::Print(Expr::Var("forward".to_string()));
        assert!(!state.write_back(&mut code));
        assert_eq!(code, "print x");
        assert!(state.write_error().is_some());
    }

    #[test]
    fn reverting_broken_edit_clears_warning() {
        let mut state = VplState::default();
        let mut code = "x = 1".to_string();
        state.sync_from_code(0, &code);

        state.blocks[0] = Statement::Let("my x".to_string(), Expr::Uint(1));
        assert!(!state.write_back(&mut code));
        assert!(state.write_error().is_some());

        state.blocks[0] = Statement::Let("x".to_string(), Expr::Uint(1));
        assert!(!state.write_back(&mut code));
        assert!(state.write_error().is_none());
        assert_eq!(code, "x = 1");
    }

    #[test]
    fn syntax_error_keeps_code_untouched() {
        let mut state = VplState::default();
        let mut code = "move forward\nloop".to_string();
        state.sync_from_code(0, &code);

        assert!(matches!(
            state.sync_status(),
            CodeSyncStatus::SyntaxError(_)
        ));
        assert!(!state.write_back(&mut code));
        assert_eq!(code, "move forward\nloop");
    }
}
