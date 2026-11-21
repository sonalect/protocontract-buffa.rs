use super::*;
#[allow(
    clippy::all,
    unused_mut,
    unused_variables,
    unused_parens,
    dead_code,
    unreachable_patterns,
    reason = "protovalidate-buffa generated validators — codegen emits uniform scaffolding regardless of which rules apply"
)]
impl ::protovalidate_buffa::Validate for Tick {
    fn validate(
        &self,
    ) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
        let mut violations: ::std::vec::Vec<::protovalidate_buffa::Violation> = ::std::vec::Vec::new();
        if let Some(v) = self.unix_ms {
            {
                #[allow(
                    unused_imports,
                    reason = "closed enums need the Enumeration trait"
                )]
                use ::buffa::Enumeration as _;
                #[allow(
                    unused_comparisons,
                    reason = "schema bounds may coincide with type limits"
                )]
                if !(((v)) >= 0i64) {
                    violations
                        .push(::protovalidate_buffa::Violation {
                            field: ::protovalidate_buffa::FieldPath {
                                elements: ::std::vec![
                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                    Some(1i32), field_name :
                                    Some(::std::borrow::Cow::Borrowed("unix_ms")), field_type :
                                    Some(::protovalidate_buffa::FieldType::Int64), key_type :
                                    None, value_type : None, subscript : None, },
                                ],
                            },
                            rule: ::protovalidate_buffa::FieldPath {
                                elements: ::std::vec![
                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                    Some(4i32), field_name :
                                    Some(::std::borrow::Cow::Borrowed("int64")), field_type :
                                    Some(::protovalidate_buffa::FieldType::Message), key_type :
                                    None, value_type : None, subscript : None, },
                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                    Some(5i32), field_name :
                                    Some(::std::borrow::Cow::Borrowed("gte")), field_type :
                                    Some(::protovalidate_buffa::FieldType::Int64), key_type :
                                    None, value_type : None, subscript : None, }
                                ],
                            },
                            rule_id: ::std::borrow::Cow::Borrowed("int64.gte"),
                            message: ::std::borrow::Cow::Borrowed(
                                "value must satisfy int64.gte",
                            ),
                            for_key: false,
                        });
                }
            }
        }
        if let Some(index) = violations
            .iter()
            .position(|v| v.rule_id == "__cel_runtime_error__")
        {
            let v = violations.swap_remove(index);
            return ::core::result::Result::Err(::protovalidate_buffa::ValidationError {
                runtime_error: ::core::option::Option::Some(v.message.into_owned()),
                ..::core::default::Default::default()
            });
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(::protovalidate_buffa::ValidationError {
                violations,
                ..::core::default::Default::default()
            })
        }
    }
}
#[allow(
    clippy::all,
    unused_mut,
    unused_variables,
    unused_parens,
    dead_code,
    unreachable_patterns,
    reason = "protovalidate-buffa generated validators — codegen emits uniform scaffolding regardless of which rules apply"
)]
impl ::protovalidate_buffa::Validate for __buffa::view::TickView<'_> {
    fn validate(
        &self,
    ) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
        let mut violations: ::std::vec::Vec<::protovalidate_buffa::Violation> = ::std::vec::Vec::new();
        if let Some(v) = self.unix_ms {
            {
                #[allow(
                    unused_imports,
                    reason = "closed enums need the Enumeration trait"
                )]
                use ::buffa::Enumeration as _;
                #[allow(
                    unused_comparisons,
                    reason = "schema bounds may coincide with type limits"
                )]
                if !(((v)) >= 0i64) {
                    violations
                        .push(::protovalidate_buffa::Violation {
                            field: ::protovalidate_buffa::FieldPath {
                                elements: ::std::vec![
                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                    Some(1i32), field_name :
                                    Some(::std::borrow::Cow::Borrowed("unix_ms")), field_type :
                                    Some(::protovalidate_buffa::FieldType::Int64), key_type :
                                    None, value_type : None, subscript : None, },
                                ],
                            },
                            rule: ::protovalidate_buffa::FieldPath {
                                elements: ::std::vec![
                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                    Some(4i32), field_name :
                                    Some(::std::borrow::Cow::Borrowed("int64")), field_type :
                                    Some(::protovalidate_buffa::FieldType::Message), key_type :
                                    None, value_type : None, subscript : None, },
                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                    Some(5i32), field_name :
                                    Some(::std::borrow::Cow::Borrowed("gte")), field_type :
                                    Some(::protovalidate_buffa::FieldType::Int64), key_type :
                                    None, value_type : None, subscript : None, }
                                ],
                            },
                            rule_id: ::std::borrow::Cow::Borrowed("int64.gte"),
                            message: ::std::borrow::Cow::Borrowed(
                                "value must satisfy int64.gte",
                            ),
                            for_key: false,
                        });
                }
            }
        }
        if let Some(index) = violations
            .iter()
            .position(|v| v.rule_id == "__cel_runtime_error__")
        {
            let v = violations.swap_remove(index);
            return ::core::result::Result::Err(::protovalidate_buffa::ValidationError {
                runtime_error: ::core::option::Option::Some(v.message.into_owned()),
                ..::core::default::Default::default()
            });
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(::protovalidate_buffa::ValidationError {
                violations,
                ..::core::default::Default::default()
            })
        }
    }
}
#[allow(
    clippy::all,
    unused_mut,
    unused_variables,
    unused_parens,
    dead_code,
    unreachable_patterns,
    reason = "protovalidate-buffa generated validators — codegen emits uniform scaffolding regardless of which rules apply"
)]
impl ::protovalidate_buffa::Validate for ClockRef {
    fn validate(
        &self,
    ) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
        let mut violations: ::std::vec::Vec<::protovalidate_buffa::Violation> = ::std::vec::Vec::new();
        if let Some(index) = violations
            .iter()
            .position(|v| v.rule_id == "__cel_runtime_error__")
        {
            let v = violations.swap_remove(index);
            return ::core::result::Result::Err(::protovalidate_buffa::ValidationError {
                runtime_error: ::core::option::Option::Some(v.message.into_owned()),
                ..::core::default::Default::default()
            });
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(::protovalidate_buffa::ValidationError {
                violations,
                ..::core::default::Default::default()
            })
        }
    }
}
#[allow(
    clippy::all,
    unused_mut,
    unused_variables,
    unused_parens,
    dead_code,
    unreachable_patterns,
    reason = "protovalidate-buffa generated validators — codegen emits uniform scaffolding regardless of which rules apply"
)]
impl ::protovalidate_buffa::Validate for __buffa::view::ClockRefView<'_> {
    fn validate(
        &self,
    ) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
        let mut violations: ::std::vec::Vec<::protovalidate_buffa::Violation> = ::std::vec::Vec::new();
        if let Some(index) = violations
            .iter()
            .position(|v| v.rule_id == "__cel_runtime_error__")
        {
            let v = violations.swap_remove(index);
            return ::core::result::Result::Err(::protovalidate_buffa::ValidationError {
                runtime_error: ::core::option::Option::Some(v.message.into_owned()),
                ..::core::default::Default::default()
            });
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(::protovalidate_buffa::ValidationError {
                violations,
                ..::core::default::Default::default()
            })
        }
    }
}
#[allow(
    clippy::all,
    unused_mut,
    unused_variables,
    unused_parens,
    dead_code,
    unreachable_patterns,
    reason = "protovalidate-buffa generated validators — codegen emits uniform scaffolding regardless of which rules apply"
)]
impl ::protovalidate_buffa::Validate for Alarm {
    fn validate(
        &self,
    ) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
        let mut violations: ::std::vec::Vec<::protovalidate_buffa::Violation> = ::std::vec::Vec::new();
        if self.label.is_none() {
            violations
                .push(::protovalidate_buffa::Violation {
                    field: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(1i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("label")), field_type :
                            Some(::protovalidate_buffa::FieldType::String), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(25i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("required")), field_type :
                            Some(::protovalidate_buffa::FieldType::Bool), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule_id: ::std::borrow::Cow::Borrowed("required"),
                    message: ::std::borrow::Cow::Borrowed("value is required"),
                    for_key: false,
                });
        }
        if self.label.is_some() {
            if let Some(v) = self.label.as_deref() {
                {
                    #[allow(
                        unused_imports,
                        reason = "closed enums need the Enumeration trait"
                    )]
                    use ::buffa::Enumeration as _;
                    if (::core::convert::AsRef::<
                        str,
                    >::as_ref(&(::core::convert::AsRef::<str>::as_ref(&(v)))))
                        .chars()
                        .count() < 1usize
                    {
                        violations
                            .push(::protovalidate_buffa::Violation {
                                field: ::protovalidate_buffa::FieldPath {
                                    elements: ::std::vec![
                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                        Some(1i32), field_name :
                                        Some(::std::borrow::Cow::Borrowed("label")), field_type :
                                        Some(::protovalidate_buffa::FieldType::String), key_type :
                                        None, value_type : None, subscript : None, },
                                    ],
                                },
                                rule: ::protovalidate_buffa::FieldPath {
                                    elements: ::std::vec![
                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                        Some(14i32), field_name :
                                        Some(::std::borrow::Cow::Borrowed("string")), field_type :
                                        Some(::protovalidate_buffa::FieldType::Message), key_type :
                                        None, value_type : None, subscript : None, },
                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                        Some(2i32), field_name :
                                        Some(::std::borrow::Cow::Borrowed("min_len")), field_type :
                                        Some(::protovalidate_buffa::FieldType::Uint64), key_type :
                                        None, value_type : None, subscript : None, }
                                    ],
                                },
                                rule_id: ::std::borrow::Cow::Borrowed("string.min_len"),
                                message: ::std::borrow::Cow::Owned(
                                    ::std::format!(
                                        "value length must be at least {} characters", 1usize
                                    ),
                                ),
                                for_key: false,
                            });
                    }
                }
            }
        }
        if !self.clock.is_set() {
            violations
                .push(::protovalidate_buffa::Violation {
                    field: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(2i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("clock")), field_type :
                            Some(::protovalidate_buffa::FieldType::Message), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(25i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("required")), field_type :
                            Some(::protovalidate_buffa::FieldType::Bool), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule_id: ::std::borrow::Cow::Borrowed("required"),
                    message: ::std::borrow::Cow::Borrowed("value is required"),
                    for_key: false,
                });
        }
        if self.clock.is_set() {
            if let Some(inner) = self.clock.as_option() {
                {
                    #[allow(
                        unused_imports,
                        reason = "closed enums need the Enumeration trait"
                    )]
                    use ::buffa::Enumeration as _;
                    if let Err(sub) = (inner).validate() {
                        if sub.compile_error.is_some() || sub.runtime_error.is_some() {
                            return Err(sub);
                        }
                        violations
                            .extend(
                                sub
                                    .violations
                                    .into_iter()
                                    .map(|mut v| {
                                        v.field
                                            .elements
                                            .splice(
                                                0..0,
                                                (::protovalidate_buffa::FieldPath {
                                                    elements: ::std::vec![
                                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                                        Some(2i32), field_name :
                                                        Some(::std::borrow::Cow::Borrowed("clock")), field_type :
                                                        Some(::protovalidate_buffa::FieldType::Message), key_type :
                                                        None, value_type : None, subscript : None, },
                                                    ],
                                                })
                                                    .elements,
                                            );
                                        v
                                    }),
                            );
                    }
                }
            }
        }
        if self.backups.len() > 3usize {
            violations
                .push(::protovalidate_buffa::Violation {
                    field: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(3i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("backups")), field_type :
                            Some(::protovalidate_buffa::FieldType::Message), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(18i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("repeated")), field_type :
                            Some(::protovalidate_buffa::FieldType::Message), key_type :
                            None, value_type : None, subscript : None, },
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(2i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("max_items")), field_type :
                            Some(::protovalidate_buffa::FieldType::Uint64), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule_id: ::std::borrow::Cow::Borrowed("repeated.max_items"),
                    message: ::std::borrow::Cow::Owned(
                        ::std::format!(
                            "list must contain at most {} items (got {})", 3usize, self
                            .backups.len()
                        ),
                    ),
                    for_key: false,
                });
        }
        for (idx, elem) in self.backups.iter().enumerate() {
            {
                #[allow(
                    unused_imports,
                    reason = "closed enums need the Enumeration trait"
                )]
                use ::buffa::Enumeration as _;
                if let Err(sub) = (elem).validate() {
                    if sub.compile_error.is_some() || sub.runtime_error.is_some() {
                        return Err(sub);
                    }
                    violations
                        .extend(
                            sub
                                .violations
                                .into_iter()
                                .map(|mut v| {
                                    v.field
                                        .elements
                                        .splice(
                                            0..0,
                                            (::protovalidate_buffa::FieldPath {
                                                elements: ::std::vec![
                                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                                    Some(3i32), field_name :
                                                    Some(::std::borrow::Cow::Borrowed("backups")), field_type :
                                                    Some(::protovalidate_buffa::FieldType::Message), key_type :
                                                    None, value_type : None, subscript :
                                                    Some(::protovalidate_buffa::Subscript::Index(idx as u64)),
                                                    },
                                                ],
                                            })
                                                .elements,
                                        );
                                    v
                                }),
                        );
                }
            }
        }
        if let Some(index) = violations
            .iter()
            .position(|v| v.rule_id == "__cel_runtime_error__")
        {
            let v = violations.swap_remove(index);
            return ::core::result::Result::Err(::protovalidate_buffa::ValidationError {
                runtime_error: ::core::option::Option::Some(v.message.into_owned()),
                ..::core::default::Default::default()
            });
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(::protovalidate_buffa::ValidationError {
                violations,
                ..::core::default::Default::default()
            })
        }
    }
}
#[allow(
    clippy::all,
    unused_mut,
    unused_variables,
    unused_parens,
    dead_code,
    unreachable_patterns,
    reason = "protovalidate-buffa generated validators — codegen emits uniform scaffolding regardless of which rules apply"
)]
impl ::protovalidate_buffa::Validate for __buffa::view::AlarmView<'_> {
    fn validate(
        &self,
    ) -> ::core::result::Result<(), ::protovalidate_buffa::ValidationError> {
        let mut violations: ::std::vec::Vec<::protovalidate_buffa::Violation> = ::std::vec::Vec::new();
        if self.label.is_none() {
            violations
                .push(::protovalidate_buffa::Violation {
                    field: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(1i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("label")), field_type :
                            Some(::protovalidate_buffa::FieldType::String), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(25i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("required")), field_type :
                            Some(::protovalidate_buffa::FieldType::Bool), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule_id: ::std::borrow::Cow::Borrowed("required"),
                    message: ::std::borrow::Cow::Borrowed("value is required"),
                    for_key: false,
                });
        }
        if self.label.is_some() {
            if let Some(v) = self.label.as_deref() {
                {
                    #[allow(
                        unused_imports,
                        reason = "closed enums need the Enumeration trait"
                    )]
                    use ::buffa::Enumeration as _;
                    if (::core::convert::AsRef::<
                        str,
                    >::as_ref(&(::core::convert::AsRef::<str>::as_ref(&(v)))))
                        .chars()
                        .count() < 1usize
                    {
                        violations
                            .push(::protovalidate_buffa::Violation {
                                field: ::protovalidate_buffa::FieldPath {
                                    elements: ::std::vec![
                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                        Some(1i32), field_name :
                                        Some(::std::borrow::Cow::Borrowed("label")), field_type :
                                        Some(::protovalidate_buffa::FieldType::String), key_type :
                                        None, value_type : None, subscript : None, },
                                    ],
                                },
                                rule: ::protovalidate_buffa::FieldPath {
                                    elements: ::std::vec![
                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                        Some(14i32), field_name :
                                        Some(::std::borrow::Cow::Borrowed("string")), field_type :
                                        Some(::protovalidate_buffa::FieldType::Message), key_type :
                                        None, value_type : None, subscript : None, },
                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                        Some(2i32), field_name :
                                        Some(::std::borrow::Cow::Borrowed("min_len")), field_type :
                                        Some(::protovalidate_buffa::FieldType::Uint64), key_type :
                                        None, value_type : None, subscript : None, }
                                    ],
                                },
                                rule_id: ::std::borrow::Cow::Borrowed("string.min_len"),
                                message: ::std::borrow::Cow::Owned(
                                    ::std::format!(
                                        "value length must be at least {} characters", 1usize
                                    ),
                                ),
                                for_key: false,
                            });
                    }
                }
            }
        }
        if !self.clock.is_set() {
            violations
                .push(::protovalidate_buffa::Violation {
                    field: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(2i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("clock")), field_type :
                            Some(::protovalidate_buffa::FieldType::Message), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(25i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("required")), field_type :
                            Some(::protovalidate_buffa::FieldType::Bool), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule_id: ::std::borrow::Cow::Borrowed("required"),
                    message: ::std::borrow::Cow::Borrowed("value is required"),
                    for_key: false,
                });
        }
        if self.clock.is_set() {
            if let Some(inner) = self.clock.as_option() {
                {
                    #[allow(
                        unused_imports,
                        reason = "closed enums need the Enumeration trait"
                    )]
                    use ::buffa::Enumeration as _;
                    if let Err(sub) = (inner).validate() {
                        if sub.compile_error.is_some() || sub.runtime_error.is_some() {
                            return Err(sub);
                        }
                        violations
                            .extend(
                                sub
                                    .violations
                                    .into_iter()
                                    .map(|mut v| {
                                        v.field
                                            .elements
                                            .splice(
                                                0..0,
                                                (::protovalidate_buffa::FieldPath {
                                                    elements: ::std::vec![
                                                        ::protovalidate_buffa::FieldPathElement { field_number :
                                                        Some(2i32), field_name :
                                                        Some(::std::borrow::Cow::Borrowed("clock")), field_type :
                                                        Some(::protovalidate_buffa::FieldType::Message), key_type :
                                                        None, value_type : None, subscript : None, },
                                                    ],
                                                })
                                                    .elements,
                                            );
                                        v
                                    }),
                            );
                    }
                }
            }
        }
        if self.backups.len() > 3usize {
            violations
                .push(::protovalidate_buffa::Violation {
                    field: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(3i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("backups")), field_type :
                            Some(::protovalidate_buffa::FieldType::Message), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule: ::protovalidate_buffa::FieldPath {
                        elements: ::std::vec![
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(18i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("repeated")), field_type :
                            Some(::protovalidate_buffa::FieldType::Message), key_type :
                            None, value_type : None, subscript : None, },
                            ::protovalidate_buffa::FieldPathElement { field_number :
                            Some(2i32), field_name :
                            Some(::std::borrow::Cow::Borrowed("max_items")), field_type :
                            Some(::protovalidate_buffa::FieldType::Uint64), key_type :
                            None, value_type : None, subscript : None, },
                        ],
                    },
                    rule_id: ::std::borrow::Cow::Borrowed("repeated.max_items"),
                    message: ::std::borrow::Cow::Owned(
                        ::std::format!(
                            "list must contain at most {} items (got {})", 3usize, self
                            .backups.len()
                        ),
                    ),
                    for_key: false,
                });
        }
        for (idx, elem) in self.backups.iter().enumerate() {
            {
                #[allow(
                    unused_imports,
                    reason = "closed enums need the Enumeration trait"
                )]
                use ::buffa::Enumeration as _;
                if let Err(sub) = (elem).validate() {
                    if sub.compile_error.is_some() || sub.runtime_error.is_some() {
                        return Err(sub);
                    }
                    violations
                        .extend(
                            sub
                                .violations
                                .into_iter()
                                .map(|mut v| {
                                    v.field
                                        .elements
                                        .splice(
                                            0..0,
                                            (::protovalidate_buffa::FieldPath {
                                                elements: ::std::vec![
                                                    ::protovalidate_buffa::FieldPathElement { field_number :
                                                    Some(3i32), field_name :
                                                    Some(::std::borrow::Cow::Borrowed("backups")), field_type :
                                                    Some(::protovalidate_buffa::FieldType::Message), key_type :
                                                    None, value_type : None, subscript :
                                                    Some(::protovalidate_buffa::Subscript::Index(idx as u64)),
                                                    },
                                                ],
                                            })
                                                .elements,
                                        );
                                    v
                                }),
                        );
                }
            }
        }
        if let Some(index) = violations
            .iter()
            .position(|v| v.rule_id == "__cel_runtime_error__")
        {
            let v = violations.swap_remove(index);
            return ::core::result::Result::Err(::protovalidate_buffa::ValidationError {
                runtime_error: ::core::option::Option::Some(v.message.into_owned()),
                ..::core::default::Default::default()
            });
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(::protovalidate_buffa::ValidationError {
                violations,
                ..::core::default::Default::default()
            })
        }
    }
}
