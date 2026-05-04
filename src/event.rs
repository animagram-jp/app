
    // ----------------------------------------------------------
    // input / submit
    // ----------------------------------------------------------

    fn on_chat_submit(&mut self, text: &str) -> Vec<DomCmd> {
        let trimmed = text.trim();
        if trimmed.is_empty() { return vec![]; }
        let cmd = self.log(RollLog::Message(trimmed.to_string()));
        vec![cmd, DomCmd::new(Operation::SetValue, "chat_input", None, Some(""))]
    }

    fn on_chat_input(&mut self, value: &str) -> Vec<DomCmd> {
        if value != "/" { return vec![]; }
        self.state = State::Selector { idx: 0 };
        let first_id = format!("roll_{}", Roll::all()[0].dom_id());
        vec![
            DomCmd::new(Operation::SetValue, "chat_input", None, Some("")),
            DomCmd::new(Operation::SetAttr,  "selector", Some("hidden"), Some("")),
            DomCmd::new(Operation::SetAttr,  "selector", Some("inert"),  Some("")),
            DomCmd::new(Operation::Focus,    &first_id,  None, None),
        ]
    }

    // ----------------------------------------------------------
    // keydown — state別に分岐
    // ----------------------------------------------------------

    fn on_keydown(&mut self, key: KeyName) -> Vec<DomCmd> {
        match &self.state {
            State::DiceInput { .. }     => self.dice_keydown(key),
            State::SkillSelector { .. } => self.skill_selector_keydown(key),
            State::CharSelector { .. }  => self.char_selector_keydown(key),
            State::Selector { .. }      => self.selector_keydown(key),
            State::Idle                 => vec![],
        }
    }

    fn selector_keydown(&mut self, key: KeyName) -> Vec<DomCmd> {
        let State::Selector { idx } = self.state else { return vec![]; };
        let all = Roll::all();
        let len = all.len();
        match key {
            KeyName::ArrowDown  => { self.state = State::Selector { idx: (idx + 1) % len };
                                     vec![DomCmd::new(Operation::Focus, &format!("roll_{}", all[idx_of(&self.state)].dom_id()), None, None)] }
            KeyName::ArrowUp    => { self.state = State::Selector { idx: (idx + len - 1) % len };
                                     vec![DomCmd::new(Operation::Focus, &format!("roll_{}", all[idx_of(&self.state)].dom_id()), None, None)] }
            KeyName::Enter  => self.on_roll_select(all[idx]),
            KeyName::Escape => self.close_selector(),
            _               => vec![],
        }
    }

    fn char_selector_keydown(&mut self, key: KeyName) -> Vec<DomCmd> {
        let State::CharSelector { idx } = self.state else { return vec![]; };
        let chars = schema::attribute(schema::Attribute::Characteristic);
        let len = chars.len();
        match key {
            KeyName::ArrowDown  => { self.state = State::CharSelector { idx: (idx + 1) % len };
                                     vec![DomCmd::new(Operation::Focus, &format!("char_roll_{}", chars[idx_of(&self.state)].dom_id()), None, None)] }
            KeyName::ArrowUp    => { self.state = State::CharSelector { idx: (idx + len - 1) % len };
                                     vec![DomCmd::new(Operation::Focus, &format!("char_roll_{}", chars[idx_of(&self.state)].dom_id()), None, None)] }
            KeyName::Enter  => self.do_char_roll(chars[idx]),
            KeyName::Escape => self.close_char_selector(),
            _               => vec![],
        }
    }

    fn skill_selector_keydown(&mut self, key: KeyName) -> Vec<DomCmd> {
        let State::SkillSelector { mode, idx } = self.state else { return vec![]; };
        let candidates = self.skill_candidates(mode);
        let len = candidates.len();
        if len == 0 { return self.close_skill_selector(); }
        match key {
            KeyName::ArrowDown  => { self.state = State::SkillSelector { mode, idx: (idx + 1) % len };
                                     vec![DomCmd::new(Operation::Focus, &candidates[idx_of(&self.state)], None, None)] }
            KeyName::ArrowUp    => { self.state = State::SkillSelector { mode, idx: (idx + len - 1) % len };
                                     vec![DomCmd::new(Operation::Focus, &candidates[idx_of(&self.state)], None, None)] }
            KeyName::Enter => { let id = candidates[idx].clone();
                                let k = id.strip_prefix("skill_roll_").unwrap_or("");
                                let field = schema::attribute(schema::Attribute::Skill).iter().find(|m| m.dom_id() == k).copied();
                                if let Some(f) = field { self.do_skill_action(mode, f) }
                                else { self.close_skill_selector() } }
            KeyName::Escape => self.close_skill_selector(),
            _               => vec![],
        }
    }

    fn dice_keydown(&mut self, key: KeyName) -> Vec<DomCmd> {
        match key {
            KeyName::Escape     => self.close_dice_input(),
            KeyName::Enter      => self.dice_advance(),
            KeyName::ArrowUp    => self.dice_increment(true),
            KeyName::ArrowDown  => self.dice_increment(false),
            _                   => vec![],
        }
    }

    // ----------------------------------------------------------
    // click — ClickTarget に変換済み
    // ----------------------------------------------------------

    fn on_click(&mut self, target: ClickTarget) -> Vec<DomCmd> {
        match target {
            ClickTarget::SelectorOverlay       => self.close_selector(),
            ClickTarget::CharSelectorOverlay   => self.close_char_selector(),
            ClickTarget::SkillSelectorOverlay  => self.close_skill_selector(),
            ClickTarget::DiceInputOverlay      => self.close_dice_input(),
            ClickTarget::DiceUp                => self.dice_increment(true),
            ClickTarget::DiceDown              => self.dice_increment(false),
            ClickTarget::DiceNext              => self.dice_advance(),
            ClickTarget::RollItem(roll)        => self.on_roll_select(roll),
            ClickTarget::CharRollItem(field)   => self.do_char_roll(field),
            ClickTarget::SkillRollItem(field)  => {
                let mode = if let State::SkillSelector { mode, .. } = self.state { mode }
                           else { return self.close_skill_selector(); };
                self.do_skill_action(mode, field)
            }
            ClickTarget::CharEditOpen          => self.open_char_edit(),
            ClickTarget::CharEditCancel        => vec![DomCmd::new(Operation::CloseModal, "char_edit", None, None)],
            ClickTarget::CharRoll              => self.on_char_roll_all(),
            ClickTarget::CharEditRoll(field)   => self.on_char_edit_roll(field),
            ClickTarget::Unknown               => vec![],
        }
    }

    fn on_focus(&mut self, target_id: &str) {
        if let State::Selector { ref mut idx } = self.state {
            let all = Roll::all();
            if let Some(i) = all.iter().position(|r| format!("roll_{}", r.dom_id()) == target_id) {
                *idx = i;
            }
        } else if let State::CharSelector { ref mut idx } = self.state {
            let chars = schema::attribute(schema::Attribute::Characteristic);
            if let Some(i) = chars.iter().position(|m| format!("char_roll_{}", m.dom_id()) == target_id) {
                *idx = i;
            }
        } else if let State::SkillSelector { mode, .. } = self.state {
            let candidates = self.skill_candidates(mode);
            if let Some(i) = candidates.iter().position(|s| s == target_id) {
                if let State::SkillSelector { ref mut idx, .. } = self.state {
                    *idx = i;
                }
            }
        }
    }

    // ----------------------------------------------------------
    // ロール種セレクタ
    // ----------------------------------------------------------

    fn on_roll_select(&mut self, roll: Roll) -> Vec<DomCmd> {
        match roll {
            Roll::DiceRoll => {
                self.state = State::DiceInput { phase: DicePhase::Count, count: 1, sides_idx: 4, modifier: 0 };
                let mut dom_cmds = vec![
                    DomCmd::new(Operation::SetAttr, "selector", Some("hidden"), Some("true")),
                    DomCmd::new(Operation::SetAttr, "selector", Some("inert"),  Some("true")),
                ];
                dom_cmds.extend(self.render_dice_input());
                dom_cmds
            }
            Roll::SkillRoll => {
                self.open_skill_selector(SkillSelectorMode::Roll, "技能判定")
            }
            Roll::CharacteristicRoll => {
                let first_id = format!("char_roll_{}", schema::attribute(schema::Attribute::Characteristic)[0].dom_id());
                self.state = State::CharSelector { idx: 0 };
                vec![
                    DomCmd::new(Operation::SetAttr, "selector",      Some("hidden"), Some("true")),
                    DomCmd::new(Operation::SetAttr, "selector",      Some("inert"),  Some("true")),
                    DomCmd::new(Operation::SetAttr, "char_selector", Some("hidden"), Some("")),
                    DomCmd::new(Operation::SetAttr, "char_selector", Some("inert"),  Some("")),
                    DomCmd::new(Operation::Focus,   &first_id,       None,           None),
                ]
            }
            Roll::PushedRoll       => self.open_skill_selector(SkillSelectorMode::Push, "プッシュロール"),
            Roll::DevelopmentCheck => self.open_skill_selector(SkillSelectorMode::DevCheck, "上達チェック"),
            roll => {
                self.state = State::Idle;
                let log_cmd = self.log(make_roll_log(roll));
                vec![
                    DomCmd::new(Operation::SetAttr, "selector", Some("hidden"), Some("true")),
                    DomCmd::new(Operation::SetAttr, "selector", Some("inert"),  Some("true")),
                    log_cmd,
                    DomCmd::new(Operation::Focus, "chat_input", None, None),
                ]
            }
        }
    }

    // ----------------------------------------------------------
    // 能力値判定セレクタ
    // ----------------------------------------------------------

    fn do_char_roll(&mut self, field: Model) -> Vec<DomCmd> {
        let label = schema::label(field, Lang::Ja);
        let difficulty = match schema::get(&self.character, field) {
            Ok(v)  => v,
            Err(_) => {
                let log_cmd = self.log(RollLog::Message(format!("[能力値判定: {}] 未入力", label)));
                let mut dom_cmds = self.close_char_selector();
                dom_cmds.push(log_cmd);
                return dom_cmds;
            }
        };
        let result = dice::skill_roll(0, Some(difficulty as u32), dice::DifficultySpec::None).unwrap();
        let entry = RollLog::Characteristic { label, difficulty, total: result.total, level: result.level };
        let log_cmd = self.log(entry);
        let mut dom_cmds = self.close_char_selector();
        dom_cmds.push(log_cmd);
        dom_cmds
    }

    // ----------------------------------------------------------
    // 技能セレクタ
    // ----------------------------------------------------------

    fn open_skill_selector(&mut self, mode: SkillSelectorMode, title: &str) -> Vec<DomCmd> {
        let candidates = self.skill_candidates(mode);
        if candidates.is_empty() {
            let msg = match mode {
                SkillSelectorMode::Roll     => "技能が未登録です",
                SkillSelectorMode::Push     => "プッシュ可能なロールがありません",
                SkillSelectorMode::DevCheck => "上達チェック対象の技能がありません",
            };
            let log_cmd = self.log(RollLog::Message(msg.to_string()));
            let mut dom_cmds = self.close_selector();
            dom_cmds.push(log_cmd);
            return dom_cmds;
        }
        self.state = State::SkillSelector { mode, idx: 0 };
        let mut dom_cmds = vec![
            DomCmd::new(Operation::SetAttr,  "selector",            Some("hidden"), Some("true")),
            DomCmd::new(Operation::SetAttr,  "selector",            Some("inert"),  Some("true")),
            DomCmd::new(Operation::SetText,  "skill_selector_title", None,          Some(title)),
            DomCmd::new(Operation::SetAttr,  "skill_selector",      Some("hidden"), Some("")),
            DomCmd::new(Operation::SetAttr,  "skill_selector",      Some("inert"),  Some("")),
        ];
        for &field in schema::attribute(schema::Attribute::Skill) {
            let id = format!("skill_roll_{}", field.dom_id());
            let visible = candidates.iter().any(|c| c == &id);
            dom_cmds.push(DomCmd::new(Operation::SetAttr, &id, Some("hidden"), Some(if visible { "" } else { "true" })));
            dom_cmds.push(DomCmd::new(Operation::SetAttr, &id, Some("inert"),  Some(if visible { "" } else { "true" })));
        }
        if !candidates.is_empty() { dom_cmds.push(DomCmd::new(Operation::Focus, &candidates[0], None, None)); }
        dom_cmds
    }

    fn do_skill_action(&mut self, mode: SkillSelectorMode, field: Model) -> Vec<DomCmd> {
        match mode {
            SkillSelectorMode::Roll     => self.do_skill_roll(field, false),
            SkillSelectorMode::Push     => {
                for entry in self.roll_log.iter_mut().rev() {
                    if let RollLog::Skill { field: f, pushed, .. } = entry {
                        if *f == field && !*pushed { *pushed = true; break; }
                    }
                }
                self.do_skill_roll(field, true)
            }
            SkillSelectorMode::DevCheck => self.do_dev_check(field),
        }
    }

    fn do_skill_roll(&mut self, field: Model, pushed: bool) -> Vec<DomCmd> {
        let difficulty = match schema::skill::get(&self.character, field) {
            Ok(v)  => v,
            Err(_) => return self.close_skill_selector(),
        };
        let label = schema::label(field, Lang::Ja);
        let result = dice::skill_roll(0, Some(difficulty as u32), dice::DifficultySpec::None).unwrap();
        let entry = RollLog::Skill { field, label, difficulty, total: result.total, level: result.level, pushed };
        let log_cmd = self.log(entry);
        let mut dom_cmds = self.close_skill_selector();
        dom_cmds.push(log_cmd);
        dom_cmds
    }

    fn do_dev_check(&mut self, field: Model) -> Vec<DomCmd> {
        let current = match schema::skill::get(&self.character, field) {
            Ok(v)  => v,
            Err(_) => return self.close_skill_selector(),
        };
        let label = schema::label(field, Lang::Ja);
        let roll = crate::n_d_n(1, 100);
        let mut dom_cmds = self.close_skill_selector();
        if roll > current as u32 {
            let gain = crate::n_d_n(1, 10) as u16;
            let new_val = current.saturating_add(gain);
            let _ = schema::skill::set(&mut self.character, field, new_val);
            let msg = format!("[上達チェック: {}] 出目: {} > {} → 成功! +{} → {}", label, roll, current, gain, new_val);
            dom_cmds.push(self.log(RollLog::Message(msg)));
            dom_cmds.push(DomCmd::new(Operation::SetText, &format!("skill_val_{}", field.dom_id()), None, Some(&new_val.to_string())));
        } else {
            let msg = format!("[上達チェック: {}] 出目: {} ≤ {} → 失敗", label, roll, current);
            dom_cmds.push(self.log(RollLog::Message(msg)));
        }
        dom_cmds
    }

    fn skill_candidates(&self, mode: SkillSelectorMode) -> Vec<String> {
        let skills = schema::attribute(schema::Attribute::Skill);
        match mode {
            SkillSelectorMode::Roll => {
                skills.iter()
                    .filter(|&&f| schema::skill::get(&self.character, f).is_ok())
                    .map(|f| format!("skill_roll_{}", f.dom_id()))
                    .collect()
            }
            SkillSelectorMode::Push => {
                self.roll_log.iter().rev()
                    .find_map(|entry| {
                        if let RollLog::Skill { field, level, pushed: false, .. } = entry {
                            let is_failure = matches!(level,
                                Some(ResultLevel::Failure) | Some(ResultLevel::Fumble) | None);
                            if is_failure {
                                return Some(vec![format!("skill_roll_{}", field.dom_id())]);
                            }
                        }
                        None
                    })
                    .unwrap_or_default()
            }
            SkillSelectorMode::DevCheck => {
                let mut eligible: Vec<Model> = Vec::new();
                for entry in &self.roll_log {
                    if let RollLog::Skill { field, level, pushed: false, .. } = entry {
                        if matches!(level,
                            Some(ResultLevel::Regular) | Some(ResultLevel::Hard) |
                            Some(ResultLevel::Extreme) | Some(ResultLevel::Critical))
                            && !eligible.contains(field)
                        {
                            eligible.push(*field);
                        }
                    }
                }
                skills.iter()
                    .filter(|f| eligible.contains(f))
                    .map(|f| format!("skill_roll_{}", f.dom_id()))
                    .collect()
            }
        }
    }

    // ----------------------------------------------------------
    // ダイス入力
    // ----------------------------------------------------------

    fn dice_advance(&mut self) -> Vec<DomCmd> {
        let State::DiceInput { phase, count, sides_idx, modifier } = self.state
            else { return vec![]; };
        match phase {
            DicePhase::Count    => { self.state = State::DiceInput { phase: DicePhase::Sides, count, sides_idx, modifier }; self.render_dice_input() }
            DicePhase::Sides    => { self.state = State::DiceInput { phase: DicePhase::Modifier, count, sides_idx, modifier }; self.render_dice_input() }
            DicePhase::Modifier => self.execute_dice_roll(),
        }
    }

    fn dice_increment(&mut self, up: bool) -> Vec<DomCmd> {
        let State::DiceInput { phase, count, sides_idx, modifier } = self.state
            else { return vec![]; };
        let len = DICE_SIDES.len();
        self.state = State::DiceInput {
            phase,
            count:     if phase == DicePhase::Count    { if up { count.saturating_add(1).min(99) } else { count.saturating_sub(1).max(1) } } else { count },
            sides_idx: if phase == DicePhase::Sides    { if up { (sides_idx + 1) % len } else { (sides_idx + len - 1) % len } } else { sides_idx },
            modifier:  if phase == DicePhase::Modifier { if up { modifier.saturating_add(1) } else { modifier.saturating_sub(1) } } else { modifier },
        };
        self.render_dice_input()
    }

    fn render_dice_input(&self) -> Vec<DomCmd> {
        let State::DiceInput { phase, count, sides_idx, modifier } = self.state
            else { return vec![]; };
        let sides = DICE_SIDES[sides_idx];
        let modifier_str = match modifier.cmp(&0) {
            std::cmp::Ordering::Greater => format!("+{}", modifier),
            std::cmp::Ordering::Less    => modifier.to_string(),
            std::cmp::Ordering::Equal   => "0".to_string(),
        };
        let next_label = if phase == DicePhase::Modifier { "ロール" } else { "次へ" };
        let hint = match phase {
            DicePhase::Count    => format!("個数: {}", count),
            DicePhase::Sides    => format!("{}個 × {}面", count, sides),
            DicePhase::Modifier => {
                let mod_part = if modifier != 0 { format!(" {}", modifier_str) } else { String::new() };
                format!("{}個 × {}面{}", count, sides, mod_part)
            }
        };
        let (h_count, h_sides, h_mod) = match phase {
            DicePhase::Count    => ("", "true", "true"),
            DicePhase::Sides    => ("true", "", "true"),
            DicePhase::Modifier => ("true", "true", ""),
        };
        vec![
            DomCmd::new(Operation::SetAttr, "dice_input",        Some("hidden"), Some("")),
            DomCmd::new(Operation::SetAttr, "dice_input",        Some("inert"),  Some("")),
            DomCmd::new(Operation::SetAttr, "dice_count_row",    Some("hidden"), Some(h_count)),
            DomCmd::new(Operation::SetAttr, "dice_sides_row",    Some("hidden"), Some(h_sides)),
            DomCmd::new(Operation::SetAttr, "dice_modifier_row", Some("hidden"), Some(h_mod)),
            DomCmd::new(Operation::SetText, "dice_count_val",    None, Some(&count.to_string())),
            DomCmd::new(Operation::SetText, "dice_sides_val",    None, Some(&format!("{}面", sides))),
            DomCmd::new(Operation::SetText, "dice_modifier_val", None, Some(&modifier_str)),
            DomCmd::new(Operation::SetText, "dice_hint",         None, Some(&hint)),
            DomCmd::new(Operation::SetText, "dice_next",         None, Some(next_label)),
            DomCmd::new(Operation::Focus,   "dice_input_focus",  None, None),
        ]
    }

    fn execute_dice_roll(&mut self) -> Vec<DomCmd> {
        let State::DiceInput { count, sides_idx, modifier, .. } = self.state
            else { return vec![]; };
        let sides = DICE_SIDES[sides_idx];
        let raw   = crate::n_d_n(count, sides);
        let total = (raw as i32 + modifier).max(0) as u32;
        let modifier_str = match modifier.cmp(&0) {
            std::cmp::Ordering::Greater => format!("+{}", modifier),
            std::cmp::Ordering::Less    => modifier.to_string(),
            std::cmp::Ordering::Equal   => String::new(),
        };
        let expr = format!("{}d{}{}", count, sides, modifier_str);
        let msg  = format!("[ダイスロール: {}] 出目: {} → 合計: {}", expr, raw, total);
        let log_cmd = self.log(RollLog::Message(msg));
        let mut dom_cmds = self.close_dice_input();
        dom_cmds.push(log_cmd);
        dom_cmds
    }


    // ----------------------------------------------------------
    // キャラクター編集
    // ----------------------------------------------------------

    fn open_char_edit(&self) -> Vec<DomCmd> {
        let ch = &self.character;
        let mut dom_cmds = vec![DomCmd::new(Operation::OpenModal, "char_edit", None, None)];
        for &field in schema::attribute(schema::Attribute::Characteristic) {
            if let Ok(v) = schema::get(ch, field) {
                dom_cmds.push(DomCmd::new(Operation::SetValue, &format!("edit_{}", field.dom_id()), None, Some(&v.to_string())));
            }
        }
        for &field in schema::attribute(schema::Attribute::Skill) {
            let occ_id = format!("skill_occ_{}", field.dom_id());
            let int_id = format!("skill_int_{}", field.dom_id());
            let occ_val = schema::skill::get(ch, field).map(|v| v.to_string()).unwrap_or_default();
            dom_cmds.push(DomCmd::new(Operation::SetValue, &occ_id, None, Some(&occ_val)));
            dom_cmds.push(DomCmd::new(Operation::SetValue, &int_id, None, Some("")));
        }
        dom_cmds
    }

    fn on_char_roll_all(&mut self) -> Vec<DomCmd> {
        if schema::roll_characteristics(&mut self.character).is_err() { return vec![]; }
        self.stat_view_cmds()
    }

    fn on_char_edit_roll(&mut self, field: Model) -> Vec<DomCmd> {
        let v = schema::roll_characteristic(field);
        let _ = schema::set(&mut self.character, field, v);
        let _ = schema::update(&mut self.character);
        let mut dom_cmds = vec![DomCmd::new(Operation::SetValue, &format!("edit_{}", field.dom_id()), None, Some(&v.to_string()))];
        dom_cmds.extend(self.stat_view_cmds());
        dom_cmds
    }

    fn on_char_edit_save(&mut self, fields: &JsValue) -> Vec<DomCmd> {
        for &field in schema::attribute(schema::Attribute::Characteristic) {
            let s = get_js_str(fields, &format!("stat_{}", field.dom_id())).unwrap_or_default();
            if !s.is_empty() {
                let v: u16 = s.trim().parse().unwrap_or(0);
                let _ = schema::set(&mut self.character, field, v);
            }
        }
        for &field in schema::attribute(schema::Attribute::Skill) {
            let occ: u16 = get_js_str(fields, &format!("occ_{}", field.dom_id())).unwrap_or_default().trim().parse().unwrap_or(0);
            let int: u16 = get_js_str(fields, &format!("int_{}", field.dom_id())).unwrap_or_default().trim().parse().unwrap_or(0);
            if occ > 0 || int > 0 {
                let base  = schema::skill::base_value(field);
                let total = base.saturating_add(occ).saturating_add(int);
                let _ = schema::skill::set(&mut self.character, field, total);
            }
        }
        let _ = schema::update(&mut self.character);
        let mut dom_cmds = vec![DomCmd::new(Operation::CloseModal, "char_edit", None, None)];
        dom_cmds.extend(self.stat_view_cmds());
        dom_cmds
    }

    fn stat_view_cmds(&self) -> Vec<DomCmd> {
        let ch = &self.character;
        let mut dom_cmds = vec![];
        for &field in schema::attribute(schema::Attribute::Characteristic) {
            if let Ok(v) = schema::get(ch, field) {
                dom_cmds.push(DomCmd::new(Operation::SetAttr,  &format!("char_view_{}", field.dom_id()), Some("hidden"), Some("")));
                dom_cmds.push(DomCmd::new(Operation::SetText,  &format!("char_val_{}", field.dom_id()),  None,           Some(&v.to_string())));
                dom_cmds.push(DomCmd::new(Operation::SetValue, &format!("edit_{}", field.dom_id()),      None,           Some(&v.to_string())));
            }
        }
        for &field in schema::attribute(schema::Attribute::Derived) {
            if let Ok(v) = schema::get(ch, field) {
                dom_cmds.push(DomCmd::new(Operation::SetAttr, &format!("char_view_{}", field.dom_id()), Some("hidden"), Some("")));
                dom_cmds.push(DomCmd::new(Operation::SetText, &format!("char_val_{}", field.dom_id()),  None,           Some(&v.to_string())));
            }
        }
        for &field in schema::attribute(schema::Attribute::Skill) {
            if let Ok(v) = schema::skill::get(ch, field) {
                dom_cmds.push(DomCmd::new(Operation::SetAttr, &format!("skill_view_{}", field.dom_id()), Some("hidden"), Some("")));
                dom_cmds.push(DomCmd::new(Operation::SetText, &format!("skill_val_{}", field.dom_id()),  None,           Some(&v.to_string())));
            }
        }
        dom_cmds
    }

    // ----------------------------------------------------------
    // close helpers
    // ----------------------------------------------------------

    fn close_selector(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            DomCmd::new(Operation::SetAttr, "selector", Some("hidden"), Some("true")),
            DomCmd::new(Operation::SetAttr, "selector", Some("inert"),  Some("true")),
            DomCmd::new(Operation::Focus,   "chat_input", None, None),
        ]
    }

    fn close_char_selector(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            DomCmd::new(Operation::SetAttr, "char_selector", Some("hidden"), Some("true")),
            DomCmd::new(Operation::SetAttr, "char_selector", Some("inert"),  Some("true")),
            DomCmd::new(Operation::Focus,   "chat_input", None, None),
        ]
    }

    fn close_skill_selector(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            DomCmd::new(Operation::SetAttr, "skill_selector", Some("hidden"), Some("true")),
            DomCmd::new(Operation::SetAttr, "skill_selector", Some("inert"),  Some("true")),
            DomCmd::new(Operation::Focus,   "chat_input", None, None),
        ]
    }

    fn close_dice_input(&mut self) -> Vec<DomCmd> {
        self.state = State::Idle;
        vec![
            DomCmd::new(Operation::SetAttr, "dice_input", Some("hidden"), Some("true")),
            DomCmd::new(Operation::SetAttr, "dice_input", Some("inert"),  Some("true")),
            DomCmd::new(Operation::Focus,   "chat_input", None, None),
        ]
    }

    // ----------------------------------------------------------
    // log
    // ----------------------------------------------------------

    fn log(&mut self, entry: RollLog) -> DomCmd {
        self.roll_log.push(entry);
        let text: String = self.roll_log.iter().map(|e| format!("{}\n", e)).collect();
        DomCmd::new(Operation::SetText, "chat_log", None, Some(&text))
    }
}

// ============================================================
// State内idxを取り出すユーティリティ
// ============================================================

fn idx_of(state: &State) -> usize {
    match state {
        State::Selector      { idx } => *idx,
        State::CharSelector  { idx } => *idx,
        State::SkillSelector { idx, .. } => *idx,
        _ => 0,
    }
}

// ============================================================
// ロール表ヘルパー
// ============================================================

fn make_roll_log(roll: Roll) -> RollLog {
    match roll {
        Roll::BoutOfMadnessRealTime => { let r = dice::roll_madness_realtime();    RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::BoutOfMadnessSummary  => { let r = dice::roll_madness_summary();     RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::FailedCastingMinor    => { let r = dice::roll_failed_casting_minor(); RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::FailedCastingMajor    => { let r = dice::roll_failed_casting_major(); RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::PhobiaTable           => { let r = dice::roll_phobia();               RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        Roll::ManiaTable            => { let r = dice::roll_mania();                RollLog::Table { kind: r.roll_type.label(Lang::Ja), roll: r.roll, label: r.label } }
        r => RollLog::Simple { kind: r.label(Lang::Ja) },
    }
}

// ============================================================
// ロール履歴
// ============================================================

enum RollLog {
    Skill          { field: Model, label: &'static str, difficulty: u16, total: u32, level: Option<ResultLevel>, pushed: bool },
    Characteristic { label: &'static str, difficulty: u16, total: u32, level: Option<ResultLevel> },
    Table          { kind: &'static str, roll: u32, label: &'static str },
    Simple         { kind: &'static str },
    Message        (String),
}

impl std::fmt::Display for RollLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Skill { label, difficulty, total, level, pushed, .. } => {
                let kind   = if *pushed { "プッシュロール" } else { "技能判定" };
                let result = level.map_or("出目のみ", |l| l.label(Lang::Ja));
                write!(f, "[{}: {}={}] 出目: {}  結果: {}", kind, label, difficulty, total, result)
            }
            Self::Characteristic { label, difficulty, total, level } => {
                let result = level.map_or("出目のみ", |l| l.label(Lang::Ja));
                write!(f, "[能力値判定: {}={}] 出目: {}  結果: {}", label, difficulty, total, result)
            }
            Self::Table { kind, roll, label } => write!(f, "[{}] {} → {}", kind, roll, label),
            Self::Simple { kind }             => write!(f, "[{}] (パラメータ入力UI未実装)", kind),
            Self::Message(s)                  => f.write_str(s),
        }
    }
}
