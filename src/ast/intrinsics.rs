
// intrinsic parsing is in literal.rs

#[derive(Debug, Clone, Copy, strum_macros::Display, strum_macros::EnumString, PartialEq)]
#[strum(serialize_all="snake_case")]
pub enum Intrinsic {
    // This is generated in situations like if parsing fails and is essentially 
    // just a NOP that returns its first argument without doing anything
    Invalid,


    // Rendering
    RenderRectangle,
    RenderCircle,
    RenderChar,
    CreateTextChars,

    // Math intrinsics
    IntAbs,
    FloatAbs,
    FloatSqrt,
    FloatSin,
    FloatCos,
    FloatTan,
    FloatAsin,
    FloatAcos,
    FloatAtan,
    FloatAtan2,
    FloatLog,
    FloatRound,
    FloatCeil,
    FloatFloor,

    // Operator intrinsics
    IntToFloat,
    DurationToFrames,
    DurationFromFrames,
    DurationFromSeconds,
    BoolToString,
    IntToString,
    FloatToString,
    BoolNot,
    IntNeg,
    FloatNeg,
    IntAdd,
    FloatAdd,
    StringAdd,
    ListJoin,
    IntSub,
    FloatSub,
    IntMul,
    FloatMul,
    IntDiv,
    FloatDiv,
    IntMod,
    FloatMod,
    IntPower,
    FloatPower,
    IntMoreThan,
    FloatMoreThan,
    IntMoreThanOrEq,
    FloatMoreThanOrEq,
    IntLessThan,
    FloatLessThan,
    IntLessThanOrEq,
    FloatLessThanOrEq,
    BoolEq,
    IntEq,
    FloatEq,
    StringEq,
    BoolNeq,
    IntNeq,
    FloatNeq,
    StringNeq,
    ListIndex,

    // String intrinsics
    StringLength,
    StringToInt,
    StringToFloat,
    StringToLowercase,
    StringToUppercase,
    StringStrip,

    // List intrinsics
    ListLength,
    ListAdd,
    ListRemove,

    // Other
    ForeverDuration,
}
