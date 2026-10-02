//! Rewriting evidence, see [super::super::evidence] for its runtime representation.

use crate::mir::{EffectKey, Type, Value, existentialization::rewrite::Rewriter};

impl Rewriter<'_> {
    pub(super) fn make_evidence(&mut self, capabilities: &[(EffectKey, Value)], rest: &Option<Value>) -> Value {
        let rest = match rest {
            Some(rest) => self.direct(rest),
            None => self.builder.null(),
        };
        if capabilities.is_empty() {
            return rest;
        }
        let count = capabilities.len();
        let node = self.builder.evidence_node(rest, count);
        for (index, (key, capability)) in capabilities.iter().enumerate() {
            let key = self.builder.effect_key(key);
            let address = self.address(capability);
            self.builder.store_evidence_entry(node, count, index, key, address);
        }
        node
    }

    pub(super) fn lookup_evidence(&mut self, evidence: &Value, key: &EffectKey, typ: &Type) -> Value {
        let evidence = self.direct(evidence);
        self.builder.lookup_evidence(evidence, key, typ)
    }
}
