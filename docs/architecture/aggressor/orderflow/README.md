# aggressor/orderflow/ — Lentilles order flow

> Sous-nœud de [`../README.md`](../README.md). Les **lentilles** calculées sur une `Bar`
> à partir des `Trade`.

## Principe commun

Chaque lentille est un **accumulateur composable** attaché à la `Bar` : on choisit
lesquelles activer à la création d'une `Period`.

Forme esquissée (détail → Phase 7) :

```
trait BarComponent {
    fn on_trade(&mut self, &Trade);   // intégrer un trade (hot path)
    fn on_close(&mut self);           // finaliser à la fermeture de la Bar
}
```

⚠️ Hot path → pas d'allocation par trade (cf. [`../../transverse/README.md`](../../transverse/README.md)).

## Les lentilles

| Lentille | Mesure | Fichier |
|---|---|---|
| **Footprint** | volume × prix × côté | [`footprint.md`](footprint.md) |
| **VolumeProfile** | volume × prix → POC / ValueArea | [`volume-profile.md`](volume-profile.md) |
| **TPO** | temps × prix | [`tpo.md`](tpo.md) |
| **Delta / CVD** | déséquilibre Buy−Sell (+ cumul inter-Bar) | [`delta-cvd.md`](delta-cvd.md) |

> `Footprint` et `VolumeProfile` partagent la donnée « volume par prix » ; `TPO` la
> remplace par « temps par prix ». À l'implémentation, on évitera de recompter trois fois.
