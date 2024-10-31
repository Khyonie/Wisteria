#![allow(dead_code)]
pub enum StyleSetting
{
    Number { value: u16 },
    TabPolicy { policy: TabPolicy },
    Boolean { value: bool },
    BracePolicy { policy: BracePlacement },
    ParenthesisPlacement { policy: ParenthesisPlacement },
    WrappingPolicy { setting: WrappingSetting, force_split: bool, indent: IndentPolicy }
}

pub enum TabPolicy 
{
    TabsOnly,
    SpacesOnly,
    Mixed
}

pub enum BracePlacement
{
    SameLine,
    NextLine,
    NextLineIndented,
    NextLineOnWrap
}

pub enum ParenthesisPlacement
{
    SameLine,
    SeparateLine,
    SeparateLineIfNotEmpty,
    SeparateLineIfWrapped,
    PreservePosition
}

pub enum BracedCodeInline
{
    IfEmpty,
    Never,
    AtMostOne,
    IfFits,
    Preserve
}

pub enum WrappingSetting
{
    DoNotWrap,
    IfNecessary,
    AlwaysFirstOthersIfNecessary,
    AllIndentAllButFirst,
    AllIndentFirstIfNecessary
}

pub enum IndentPolicy
{
    Default,
    ByOne,
    OnColumn
}
