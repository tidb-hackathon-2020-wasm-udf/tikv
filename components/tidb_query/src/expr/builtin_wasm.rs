use super::{EvalContext, Result, ScalarFunc};
use crate::codec::Datum;

impl ScalarFunc {
    pub fn nbody<'a, 'b: 'a>(
        &'b self,
        ctx: &mut EvalContext,
        row: &'a [Datum],
    ) -> Result<Option<f64>> {
        let input = try_opt!(self.children[0].eval_int(ctx, row));
        if let Some(wasm) = ctx.wasm_store.get(self.wasm_udf_id)? {
            let res = wasm.execute("udf_main", vec![input.to_string()])?;
            if let Some(v) = res.as_ref()[0].f64() {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    // pub fn wasm_call(&self, ctx: &mut EvalContext, row: &[Datum]) -> Result<Option<()>> {
    //     Ok(None)
    // }
}
