# TODO

- [x] Allow running multiple simulation steps in one compute dispatch
- [ ] Rewrite using my compute lib?
- [ ] Audio
  - [ ] Make pickup location configurable
  - [ ] Multiple pickups?
  - [x] Make audio input/outputs configurable
  - [ ] Allow audio in/out without the other
  - [ ] Configurable sample rates
  - [ ] Configurable pause points
- [ ] Change wavespeed with light wavelength. See [Sellmeier equation](https://en.wikipedia.org/wiki/Sellmeier_equation)
- [ ] Update other configs to use correct units

```plain
n = c/v
v = (299,792,458 m/s)/n

n^2 = 1 + (b₁λ²)/(λ²-C₁) + (b₂λ²)/(λ²-C₂) + (b₃λ²)/(λ²-C₃)

B₁ = 1.03961212
B₂ = 1.03961212
B₃ = 1.01046945

C₁ = 6.00069867×10¯³ μm²
C₂ = 2.00179144×10¯² μm²
C₃ = 1.03560653×10² μm²
```

```rust
const EXPONENTS: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
```
