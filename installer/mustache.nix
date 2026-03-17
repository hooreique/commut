templateText: attrs:
let
  trim =
    s:
    let
      matched = builtins.match " *([^ ].*[^ ]|[^ ]) *" s;
    in
    if matched == null then "" else builtins.elemAt matched 0;
  renderPart =
    part:
    if builtins.isList part then
      if builtins.length part == 0 then
        ""
      else
        let
          token = builtins.head part;
          matched = builtins.match "[{][{] *([^}]+) *[}][}]" token;
          name = if matched == null then null else trim (builtins.elemAt matched 0);
          value = if name == null || !builtins.hasAttr name attrs then null else builtins.getAttr name attrs;
        in
        if value == null then token else toString value
    else
      part;
  parts = builtins.split "([{][{] *[^}]+ *[}][}])" templateText;
in
builtins.concatStringsSep "" (map renderPart parts)
