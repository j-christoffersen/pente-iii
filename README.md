# Pente III

## Dev

**After any Rust changes**, rebuild the WASM bundle:
```bash
./ui/build-wasm.sh
```

**Run the web dev server:**
```bash
cd web && npm run dev
```
Then open http://localhost:3000/game/

---

**Native build** (engine only, no WASM):
```bash
cd web && npm run build:engine
```
