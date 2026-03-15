
// // much of this is modelled after
// // * https://rustc-dev-guide.rust-lang.org/mir/index.html
// // * https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.StatementKind.html
// // * https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.TerminatorKind.html
// // * https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/mir/enum.Operand.html

// #[derive(Debug)]
// pub struct IrVariableId;

// #[derive(Debug)]
// pub struct IrBlockId;

// #[derive(Debug)]
// pub enum ConstValue {
//     Int(i64),
//     Float(f64),
//     String(String),
//     Function(IrBlockId),
// }

// #[derive(Debug)]
// pub enum IrOperand {
//     Copy(IrVariableId),
//     Move(IrVariableId),
//     Const(ConstValue),
// }

// #[derive(Debug)]
// pub enum IrRValue {
//     Use(IrOperand),
//     Ref(IrOperand),
//     // An optimization I could do is have `std::ops::add` and such for built-in 
//     // types compile into a `BinOp(IrOperand, IrOperand)` here instead of 
//     // requiring a whole function call
// }

// #[derive(Debug)]
// pub enum IrStatement {
//     Assign(IrVariableId, IrRValue),
//     /// I assume the reason rustc has this be a terminator instead is because 
//     /// Drop in Rust may fail in all sorts of ways, but for this language if 
//     /// dropping fails then we can safely assume the Rapture has started or smth
//     Drop(IrVariableId),
//     /// Await a duration in frames
//     AwaitDuration(u64),
// }

// #[derive(Debug)]
// pub enum IrTerminator {
//     Goto(IrBlockId),
//     /// Go to a specific block based on the value of the operand. The operand 
//     /// must evalute to an int, where zero means first one and non-zero means 
//     /// second one
//     /// 
//     /// In the future, if I run into performance issues with `match` statements, 
//     /// this should be changed to a `Switch` (a la Rust)
//     Either(IrOperand, IrBlockId, IrBlockId),
//     /// Function call, jumps to the block that the operand evaluates to, and 
//     /// eventually returns with the value in the specified block
//     Call(IrOperand, Vec<IrOperand>, IrBlockId),
//     /// Return from the current function
//     Return,
//     /// This block will await forever after this point
//     InfiniteDuration,
//     /// This block will never reach this terminator (such as an infinite while 
//     /// loop). If this does actually get reached, something is wrong
//     Never,
// }

// #[derive(Debug)]
// pub struct IrBlock {
//     statements: Vec<IrStatement>,
//     terminator: IrTerminator,
//     // Maybe store the calculated duration here for optimization reasons? So we 
//     // don't need to recalculate it every time?
// }
