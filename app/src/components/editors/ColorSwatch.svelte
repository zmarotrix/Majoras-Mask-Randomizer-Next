<script lang="ts">
  // A colour button that opens the system colour picker.
  import { fromHex, toHex } from "../../lib/colors";

  interface Props {
    /** "R, G, B" */
    value: unknown;
    onchange: (rgb: string) => void;
    label: string;
    disabled?: boolean;
  }
  let { value, onchange, label, disabled = false }: Props = $props();
</script>

<label class="swatch" class:disabled title={disabled ? `${label} (default)` : `${label}: ${toHex(value)}`}>
  <input type="color" value={toHex(value)} {disabled} aria-label={label} onchange={(e) => onchange(fromHex(e.currentTarget.value))} />
  <span style:background={toHex(value)}></span>
</label>

<style>
  .swatch {
    position: relative;
    display: inline-block;
    width: 34px;
    height: 24px;
    flex: none;
    cursor: pointer;
  }
  .swatch input {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: inherit;
  }
  .swatch span {
    display: block;
    width: 100%;
    height: 100%;
    border: 1px solid var(--border-strong);
    border-radius: 5px;
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.25);
  }
  .swatch:focus-within span {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .disabled {
    cursor: default;
    opacity: 0.4;
  }
</style>
