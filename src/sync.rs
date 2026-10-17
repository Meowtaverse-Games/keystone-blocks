use keystone_lang::{Error, parse};

use crate::code::statements_to_string;
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
    code: String,
    generated: String,
    status: CodeSyncStatus,
}

impl VplState {
    pub fn load_code(&mut self, source_id: usize, code: &str) {
        let sync = &mut self.sync;
        sync.source_id = Some(source_id);
        sync.code = code.to_string();

        match parse(code) {
            Ok(statements) => {
                sync.generated = statements_to_string(&statements, 0);
                sync.status = CodeSyncStatus::Synced;
                self.blocks = statements;
            }
            Err(err) => {
                sync.generated.clear();
                sync.status = CodeSyncStatus::SyntaxError(format_error(&err));
                self.blocks.clear();
            }
        }
    }

    pub fn sync_status(&self) -> &CodeSyncStatus {
        &self.sync.status
    }

    pub(crate) fn sync_from_code(&mut self, source_id: usize, code: &str) {
        if self.sync.source_id != Some(source_id) || self.sync.code != code {
            self.load_code(source_id, code);
        }
    }

    pub(crate) fn write_back(&mut self, code: &mut String) -> bool {
        if self.sync.status != CodeSyncStatus::Synced {
            return false;
        }

        let generated = statements_to_string(&self.blocks, 0);
        if generated == self.sync.generated {
            return false;
        }

        *code = generated.clone();
        self.sync.code = generated.clone();
        self.sync.generated = generated;
        true
    }
}

fn format_error(err: &Error) -> String {
    match err {
        Error::SyntaxError { messages } => messages.join("\n"),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            ),
        ]
    }

    #[test]
    fn blocks_round_trip_through_code() {
        let blocks = sample_blocks();
        let code = statements_to_string(&blocks, 0);
        assert_eq!(parse(&code).unwrap(), blocks, "generated code:\n{code}");
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
