## Bab 9  Nutrien dan karbon tanah

Bab ini membahas seluruh siklus nitrogen (N), fosfor (P), dan karbon (C) tanah di tingkat HRU: inisialisasi kolam N/P/C dari data tanah, dekomposisi residu permukaan dan residu dalam tanah, mineralisasi humus, denitrifikasi, nitrifikasi dan volatilisasi amonium, keseimbangan P mineral (labil, aktif, stabil), erosi dan nutrien yang terbawa sedimen beserta *enrichment ratio*, pencucian nitrat dan P terlarut, pencampuran tanah (biologis akhir tahun, pengolahan tanah, *puddling* sawah), serta model karbon SWAT-C (`cswat` 2) dengan kolam humus lambat/pasif, mikroba, dan serasah struktural/metabolik. File Rust yang dicakup adalah `crates/swat-engine/src/hru/nutrient.rs` (model karbon asli, `cswat` 0, plus erosi) dan `crates/swat-engine/src/hru/carbon.rs` (model karbon `cswat` 2), ditambah catatan dari `docs/nutrients.md` (model P tanah dinamis), `docs/swat-quirks.md`, dan `docs/swat-fixes.md`.

Posisinya di alur harian HRU (`hru/mod.rs`) adalah sebagai berikut. Setelah limpasan permukaan dihitung, `erosion` menghitung debit puncak dan hasil sedimen. Setelah perkolasi, untuk `cswat` 0 dipanggil `rsd_decomp` lalu `nut_nminrl`; untuk `cswat` 2 dipanggil pencampuran biologis harian (`biomix`, yang memakai `biomix_pools`), lalu `cbn_surfrsd_decomp`, `cbn_rsd_transfer`, dan `cbn_zhang2`. Berikutnya `nut_nitvol`, lalu `nut_pminrl2` (bila `sol_p_model` 1) atau `nut_pminrl`, lalu biozona septik dan pertumbuhan tanaman. Sesudah tanaman tumbuh, pada hari dengan limpasan, debit puncak, dan presipitasi, `sediment_nutrients` (dan `nut_orgnc2` untuk `cswat` 2) melepas N/P organik bersama sedimen; kemudian deposisi atmosfer (`nut_nrain`) ditambahkan ke lapisan atas, lalu `nut_nlch` dan `nut_solp`. Inisialisasi (`nutcarb_init`, `carbon_init`) dipanggil sekali per HRU dari `nutcarb_stage` setelah kalibrasi, dan pencampuran akhir tahun (`year_end_mixing`) dipanggil pada hari terakhir tahun untuk `cswat` 0. Pengolahan tanah dipanggil dari operasi manajemen (`hru/mgt.rs`).

Satuan umum: massa kolam nutrien dan karbon dalam kg/ha per lapisan; kedalaman lapisan `d` (kedalaman dasar dari permukaan) dan tebal `thick` dalam mm; air tanah `st`, `fc`, `ul`, `wpmm` dalam mm; `bd` dalam Mg/m3; `cbn`, `clay`, `silt`, `sand`, `rock` dalam persen; suhu dalam °C. Struktur `OrgMass` (dari `hru/plant.rs`) adalah empat-tuple massa organik {m, c, n, p} dalam kg/ha dengan operasi `+`, `-`, dan `OrgMass::scale(k, o)` yang mengalikan keempat komponen dengan k. Semua perhitungan memakai f32 seperti Fortran (kecuali disebut lain) sehingga urutan operasi dipertahankan demi kesamaan bit-for-bit.

### 9.1  Struktur data, parameter cekungan, dan fungsi pembantu (`nutrient.rs`)

Tiga struktur data menopang modul ini.

| Struktur | Padanan SWAT+ | Field utama (satuan) |
| --- | --- | --- |
| `NutBasin` | `bsn_prm` (parameters.bsn), `bsn_cc` (codes.bsn) | `cmn` laju mineralisasi humus aktif (1/hari); `cdn` koefisien laju denitrifikasi; `sdnco` ambang faktor air untuk denitrifikasi; `psp` indeks ketersediaan P (rasio sorpsi); `phoskd` koefisien partisi P tanah (m3/Mg); `pperco` koefisien perkolasi P; `nperco` koefisien perkolasi/ekstraksi nitrat ke limpasan; `nperco_lchtile` koefisien nitrat ke drainase; `decr_min` laju dekomposisi residu minimum (1/hari); `prf` faktor laju puncak; `rsd_covco` koefisien penutupan residu; `sed_det` metode debit puncak; `sol_p_model` (1 = P dinamis); `n_act_frac` fraksi N humus aktif (0,02 bawaan); `fixes` saklar `--fix-swat` |
| `SoilTest` | `soiltest_db` (nutrients.sol) | `exp_co` koefisien penurunan dengan kedalaman (1/mm); `lab_p` P labil permukaan (mg/kg); `nitrate` nitrat permukaan (mg/kg); `fr_hum_act` fraksi humus aktif; `hum_c_n`, `hum_c_p` rasio C:N dan C:P humus |
| `NutDay` | `hnb_d`, `hls_d` (sebelum lag) | `no3atmo`, `nh4atmo` deposisi; `tileno3`, `tilelabp` nitrat dan P labil di drainase; `act_nit_n` N mineralisasi humus; `org_lab_p` P mineralisasi humus; `act_sta_n` aliran N aktif-stabil; `denit`; `lab_min_p`, `act_sta_p` aliran P mineral; `qp_cms` debit puncak (m3/s); `usle_ei` indeks erosivitas; `usle` erosi USLE (t/ha); `sedyld` hasil sedimen (t); `enratio` enrichment ratio; `sedorgn`, `sedorgp`, `sedminpa`, `sedminps` N/P di sedimen (kg/ha); `surqno3`, `latno3`, `percn` nitrat di limpasan, aliran lateral, perkolasi; `surqsolp`, `lchlabp` P terlarut di limpasan dan tercuci (kg/ha) |

Semua field `NutDay` adalah f32 dan dimulai nol setiap hari (`NutDay::default()`).

#### 9.1.1  `NutBasin::new`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub fn new(project: &swat_model::Project) -> NutBasin`
- **Padanan SWAT+:** pembacaan `parameters.bsn` dan `codes.bsn` (`basin_read_prm`, `basin_read_cc`), serta `n_act_frac` dari `carbon.bsn`.
- **Tujuan:** Mengumpulkan parameter cekungan yang dipakai rutin nutrien dan erosi ke satu struktur yang ringan dan `Copy`.
- **Masukan dan keluaran:** `project.params` (nama parameter `parameters.bsn`), `project.codes` (`sed_det`, `sol_p_model`, `cswat`), `project.carbon`. Keluaran: `NutBasin`.
- **Cara kerja:**
  1. Setiap parameter riil dibaca dengan nama; parameter yang tidak ada bernilai 0.
  2. `n_act_frac` diambil dari tabel `carbon.bsn` hanya bila `cswat` = 2 dan tabel memuatnya; selain itu 0,02.
  3. `sed_det` dan `sol_p_model` disalin dari kode cekungan; `fixes` diset `false` (diubah kemudian oleh runner bila `--fix-swat` aktif).
- **Kebiasaan SWAT+ / catatan port:** Dengan `cswat` 0 fraksi N aktif selalu 0,02 seperti nilai tetap di `nut_nminrl` Fortran, sehingga keluaran sama dengan SWAT+.

#### 9.1.2  `SoilTest::default`

- **Lokasi:** `nutrient.rs`, `impl Default for SoilTest`
- **Padanan SWAT+:** nilai bawaan tipe `soiltest_db` di `soil_data_module`.
- **Tujuan:** Nilai uji tanah yang dipakai bila entri `soil_plant_init` tidak menemukan baris di `nutrients.sol` (atau file itu tidak ada, seperti pada Ames).
- **Masukan dan keluaran:** tidak ada masukan; `exp_co` = 0,001 1/mm, `lab_p` = 5 mg/kg, `nitrate` = 7 mg/kg, `fr_hum_act` = 0,02, `hum_c_n` = 10, `hum_c_p` = 80.
- **Kebiasaan SWAT+ / catatan port:** `docs/swat-quirks.md` mencatat bahwa `nutrients.sol` yang hilang berarti nilai bawaan SWAT+: 7 ppm nitrat dan 5 ppm P labil di permukaan, menurun dengan kedalaman. Pemanggil (`nutcarb_stage` di `hru/mod.rs`) juga mengganti `exp_co` > 0,005 dengan 0,001, sebagaimana Fortran.

#### 9.1.3  `sut_cdg`

- **Lokasi:** `nutrient.rs`, `fn sut_cdg(st: f32, fc: f32, tmp: f32) -> (f32, f32, f32)` (privat)
- **Padanan SWAT+:** kode sebaris yang berulang di `nut_nminrl.f90` dan `rsd_decomp.f90`; tidak ada subrutin tersendiri.
- **Tujuan:** Menghitung faktor siklus air tanah `sut`, faktor suhu `cdg`, dan faktor gabungan `csf` yang mengatur laju dekomposisi dan mineralisasi.
- **Masukan dan keluaran:** `st` air tanah tersedia (mm), `fc` kapasitas lapang tersedia (mm), `tmp` suhu lapisan (°C). Keluaran (`sut`, `cdg`, `csf`), tanpa satuan.
- **Cara kerja:**
  1. Faktor air, dibatasi bawah 0,05:

```latex
\gamma_{sw} = \max\left(0.05,\; 0.1 + 0.9\sqrt{\frac{SW}{FC}}\right)
```

2. Faktor suhu, dibatasi bawah 0,1:

```latex
\gamma_{tmp} = \max\left(0.1,\; \frac{0.9\,T}{T + \exp(9.93 - 0.312\,T)} + 0.1\right)
```

3. Faktor gabungan, dengan hasil kali dijepit ke \[0, 10^6\] sebelum diakarkan:

```latex
\gamma_{c} = \sqrt{\min(10^6, \max(0, \gamma_{tmp}\,\gamma_{sw}))}
```

- **Kebiasaan SWAT+ / catatan port:** Bentuk ini adalah varian SWAT+ dari Neitsch et al. (2011) persamaan mineralisasi (dalam SWAT 2009 faktor gabungan adalah akar dari hasil kali, dengan faktor air SW/FC); konstanta 0,1 + 0,9 dan batas bawahnya mengikuti kode Fortran, bukan teori.

#### 9.1.4  `residue_ca`

- **Lokasi:** `nutrient.rs`, `fn residue_ca(r: &OrgMass) -> f32` (privat)
- **Padanan SWAT+:** kode sebaris di `rsd_decomp.f90` dan `nut_nminrl.f90` (variabel `cnrf`, `cprf`, `ca`).
- **Tujuan:** Faktor rasio nutrien residu yang menurunkan laju dekomposisi bila residu miskin N atau P.
- **Masukan dan keluaran:** residu `r` (kg/ha C, N, P). Keluaran `ca` ∈ (0, 1\].
- **Cara kerja:**
  1. Bila `r.n` > 10^-4: rasio C:N dibatasi 500 dan

```latex
\gamma_{CN} = \exp\left(-0.693\,\frac{\min(500, C/N) - 25}{25}\right)
```

Bila `r.n` ≤ 10^-4, γ\_CN = 1.

2. Bila `r.p` > 10^-4: rasio C:P dibatasi 5000 dan

```latex
\gamma_{CP} = \exp\left(-0.693\,\frac{\min(5000, C/P) - 200}{200}\right)
```

Bila `r.p` ≤ 10^-4, γ\_CP = 1.

3. Hasil: `ca = min(γ_CN, γ_CP, 1)`.

- **Kebiasaan SWAT+ / catatan port:** Di model karbon (`cbn_surfrsd_decomp`) rumus yang sama dipakai dengan batas dan acuan dari `carbon.bsn` (`cnr_cap`, `cnr_ref`, `cpr_cap`, `cpr_ref`), yang nilai bawaannya sama dengan konstanta di sini.

### 9.2  Inisialisasi kolam N, P, dan C tanah

Kedua fungsi di subbab ini adalah porting `soil_nutcarb_init.f90`, yang di SWAT+ dipanggil dari program utama setelah `proc_cal` (yakni setelah sifat tanah dikalibrasi), HRU demi HRU. Di Rust pemanggilnya adalah `nutcarb_stage` (`hru/mod.rs`), yang mencari entri `soil_plant_init` HRU, mengambil baris `nutrients.sol` yang sesuai sebagai `SoilTest`, lalu memanggil `nutcarb_init` dan, bila `cswat` = 2, `carbon_init`. Bagian mineral (N dan P) selalu dihitung; kolam humus model lama hanya untuk `cswat` 0, dan kolam model karbon hanya untuk `cswat` 2.

Faktor konversi yang dipakai berulang kali adalah massa tanah per ppm:

```latex
w_{conv} = \frac{\rho_b\,h}{100}\quad[\mathrm{kg\,ha^{-1}\,per\,mg\,kg^{-1}}]
```

dengan ρ\_b bobot isi (Mg/m3) dan h tebal lapisan (mm); massa tanah satu lapisan adalah 10^4 ρ\_b h kg/ha, sehingga 1 mg/kg setara ρ\_b h / 100 kg/ha.

#### 9.2.1  `nutcarb_init`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub(super) fn nutcarb_init(&mut self, t: &SoilTest, psp_bsn: f32, cswat: i32, sol_p_model: i32, psp_saved: &mut f32)`
- **Padanan SWAT+:** `soil_nutcarb_init` (`soil_nutcarb_init.f90`), bagian nitrat, P mineral, dan humus `hact`/`hsta`.
- **Tujuan:** Mengisi kolam nitrat, P labil, P aktif, P stabil setiap lapisan dari uji tanah, dan (untuk `cswat` 0) kolam humus aktif dan stabil dari karbon organik tanah.
- **Masukan dan keluaran:** `t` (uji tanah), `psp_bsn` (PSP dari `parameters.bsn`, bawaan 0,4), `cswat`, `sol_p_model`, `psp_saved` (PSP terakhir yang tersimpan antarpanggilan). Menulis `layers[k].no3`, `.lab`, `.act`, `.sta` (kg/ha), dan untuk `cswat` 0 `.hact`, `.hsta` (OrgMass, kg/ha); `cbn` dijepit ke minimum 0,001 %.
- **Cara kerja:**
  1. Setiap lapisan: `cbn = max(cbn, 0,001)`.
  2. Untuk setiap lapisan dengan kedalaman dasar `d` (mm), faktor kedalaman dan konsentrasi awal:

```latex
f_{dep} = \exp(-k_{exp}\,d),\qquad NO_3 = c_{NO_3}\,f_{dep}\,w_{conv},\qquad P_{lab} = c_{P}\,f_{dep}\,w_{conv}
```

bila `nitrate` atau `lab_p` uji tanah ≤ 10^-9, dipakai 7 atau 5 mg/kg. 3. Rasio sorpsi P (PSP). Bila `sol_p_model` = 1, untuk lapisan yang berlempung (`clay` > 0) dengan P larut dalam mg/kg (`solp = P_lab / w_conv`) dan karbon organik C (%):

```latex
PSP = -0.045\ln(clay) + 0.001\,solp - 0.035\,C + 0.43,\qquad 0.10 \le PSP \le 0.7
```

Lapisan tanpa liat memakai `psp_saved`, dan hasil setiap lapisan disimpan kembali ke `psp_saved`. Bila `sol_p_model` ≠ 1, PSP = `psp_bsn`.

(4) P aktif mineral dari keseimbangan sorpsi:

```latex
P_{act} = P_{lab}\,\frac{1 - PSP}{PSP}
```

5. P stabil. Model P dinamis (White et al. 2009, dari Sharpley 2004) memakai rasio stabil-aktif dari total P mineral dalam mg/kg, dijepit \[1, 7\]:

```latex
SSP = 25.044\,(P_{act}^{ppm} + P_{lab}^{ppm})^{-0.3833},\qquad P_{sta} = SSP\,(P_{act} + P_{lab})
```

Model standar: `P_sta = 4 P_act`.

(6) Bila `cswat` ≠ 0 selesai. Untuk `cswat` 0, fraksi humus aktif `fr = fr_hum_act` (0,02 bila ≤ 10^-9), dan per lapisan:

```latex
M_{tot} = 10^4\,h\,\rho_b\,\frac{C}{100}\,\frac{1}{0.58},\qquad C_{tot} = 10^4\,h\,\rho_b\,\frac{C}{100}
```

```latex
hact = fr\,(M, C),\; hact.n = \frac{hact.c}{CN_{hum}},\; hact.p = \frac{hact.c}{CP_{hum}};\quad hsta = (1-fr)\,(M, C),\; hsta.n = \frac{hsta.c}{CN_{hum}},\; hsta.p = \frac{hsta.c}{CP_{hum}}
```

- **Kebiasaan SWAT+ / catatan port:**
  - `psp` adalah variabel lokal bersimpanan (*saved*) di Fortran. Lapisan tanpa liat mewarisi PSP lapisan sebelumnya, atau, untuk lapisan pertama, PSP lapisan terakhir HRU sebelumnya. Port meniru ini dengan `psp_saved` yang dibawa `nutcarb_stage` lintas HRU (`docs/nutrients.md`).
  - Faktor 0,58 adalah konversi C organik ke bahan organik (faktor van Bemmelen, 1/0,58 ≈ 1,72).
  - N dan P total (`tot.n = C/10`, `tot.p = C/100`) dihitung lalu tidak dipakai; N/P humus memakai rasio uji tanah.
  - Batas SSP pada inisialisasi adalah \[1, 7\], sedangkan harian di `nut_pminrl2` \[0,7, 10\].

#### 9.2.2  `carbon_init`

- **Lokasi:** `crates/swat-engine/src/hru/carbon.rs`, `pub(super) fn carbon_init(&mut self, cb: &CarbonBasin)`
- **Padanan SWAT+:** `soil_nutcarb_init` (`soil_nutcarb_init.f90`), cabang `cswat == 2` yang mengisi `soil1(j)%hp`, `hs`, `microb`, `meta`, `str`, `lig`, `tot`, `seq`.
- **Tujuan:** Membagi karbon organik setiap lapisan ke kolam model karbon SWAT-C: humus pasif (HP), humus lambat (HS), biomassa mikroba (BM), serasah metabolik dan struktural (dengan bagian lignin dan non-lignin).
- **Masukan dan keluaran:** `cbn` (%), `thick` (mm), `bd`, `clay`, `silt`; parameter `frac_seq` (fraksi C yang “terikat”/humus, 0,95), `frac_hum_passive` (0,44), `frac_hum_slow` (0,54), `frac_hum_microb` (0,02), `mathers`. Keluaran: `layers[k].cp` (struktur `Pools`, kg/ha).
- **Cara kerja:**
  1. Massa total seperti `nutcarb_init`: `M_tot = 10^4 h ρ_b (C/100) / 0,58`, `C_tot = 10^4 h ρ_b C/100`.
  2. Humus pasif: `hp = f_seq f_hp (M, C)`, C:N 10, C:P 80.
  3. Humus lambat: tanpa opsi Mathers `hs = f_seq f_hs (M, C)`; dengan opsi Mathers (`mathers_int` = 1), fraksi lambat dari liat+debu:

```latex
f_{M} = \begin{cases} (0.41 + 0.0053\cdot 100\,(cl+si))/100 & cl+si \ge 0.35 \\ (0.069 + 0.015\cdot 100\,(cl+si))/100 & \text{lainnya} \end{cases},\quad hs.c = f_{seq} f_M C_{tot},\; hs.m = hs.c/0.58
```

C:N 10, C:P 80.

(4) Mikroba: `microb = f_seq f_bm (M, C)`, C:N 8, C:P 80.

(5) Serasah (bagian tak terikat, 1 − f\_seq): metabolik 85 % (C:N 10, C:P 100), struktural 15 % (C:N 150, C:P 1500). Lignin = 0,8 dari massa dan C struktural, 0,2 dari N dan 0,02 dari P struktural; non-lignin = struktural − lignin.

(6) `tot = str + meta + hs + hp + microb`; `seq = hs + hp + microb`.

- **Kebiasaan SWAT+ / catatan port:** Pada opsi Mathers, `clay` dan `silt` di sini sudah dalam persen tetapi dibandingkan dengan 0,35 dan dikali 100 lagi (seperti Fortran), sehingga praktis selalu memakai cabang pertama dan fraksinya bisa melebihi 1; port mempertahankannya. Lignin tidak termasuk dalam `tot` secara terpisah karena sudah bagian dari struktural.

### 9.3  Siklus N model karbon asli (`cswat` 0): dekomposisi residu, mineralisasi, denitrifikasi, nitrifikasi, volatilisasi

Tiga fungsi di subbab ini menjalankan transformasi N harian pada model karbon asli SWAT (Neitsch et al. 2011, bab 3:1). Kolam N yang terlibat per lapisan: nitrat `no3`, amonium `nh4`, N organik humus aktif `hact.n` dan stabil `hsta.n`, serta residu segar per tanaman (`plants[i].rsd` untuk permukaan dan `plants[i].soil_rsd[k]` untuk lapisan k). Kolam P organik yang terlibat: `hact.p`, `hsta.p`, dan P residu. `rsd_decomp` dan `nut_nminrl` hanya dipanggil bila `cswat` = 0; `nut_nitvol` dipanggil untuk kedua model.

#### 9.3.1  `rsd_decomp`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub(super) fn rsd_decomp(&mut self, b: &NutBasin)`
- **Padanan SWAT+:** `rsd_decomp` (`rsd_decomp.f90`).
- **Tujuan:** Mendekomposisi residu permukaan setiap tanaman; N dan P yang dilepas masuk ke lapisan tanah teratas.
- **Masukan dan keluaran:** air (`st`, `fc`, mm) dan suhu (`tmp`, °C) lapisan 1; `rsdco_pl` (laju dekomposisi residu tanaman, 1/hari, dari `plants.plt`); `decr_min`. Mengubah `plants[i].rsd`, `layers[0].no3`, `.lab`, `.hact.n`, `.hact.p`, dan menghitung ulang `rsd_tot` (jumlah residu permukaan semua tanaman).
- **Cara kerja:**

(1) Bila suhu lapisan 1 ≤ 0 °C, residu tanaman itu dilewati. Bila `st` < 0 dipakai 10^-7.

(2) Faktor rasio nutrien `ca` = `residue_ca(rsd)` dan faktor lingkungan `csf` dari `sut_cdg` (lapisan 1).

(3) Laju dekomposisi harian, dijepit ke \[`decr_min`, 1\]:

```latex
\delta_{ntr} = \min\left(1, \max\left(\delta_{min},\; \beta_{rsd}\,\gamma_{ntr}\,\gamma_c\right)\right)
```

(4) Massa yang terdekomposisi `decomp = δ · rsd` (empat komponen m, c, n, p) dikurangkan dari residu.

(5) Pembagian hasil ke lapisan 1:

```latex
NO_3 \mathrel{+}= 0.8\,N_{dec},\quad hact.n \mathrel{+}= 0.2\,N_{dec},\quad P_{lab} \mathrel{+}= 0.8\,P_{dec},\quad hact.p \mathrel{+}= 0.2\,P_{dec}
```

(6) `rsd_tot` dijumlahkan ulang dari residu semua tanaman.

- **Kebiasaan SWAT+ / catatan port:**
  - Di sini 20 % P dimasukkan ke `hact.p`, sedangkan `nut_nminrl` untuk residu tanah memasukkannya ke `hsta.p`; keduanya mengikuti Fortran.
  - Tidak ada pembulatan nilai kecil ke nol pada residu permukaan (berbeda dengan residu tanah di `nut_nminrl`).
  - `docs/swat-quirks.md`: keluaran `rsd_nitorg_n` dan `rsd_laborg_p` tercetak 0 karena `rsd_decomp` menjumlahkannya lalu `nut_nminrl`, yang dipanggil tepat sesudahnya, menolkannya.

#### 9.3.2  `nut_nminrl`

- **Lokasi:** `nutrient.rs`, `pub(super) fn nut_nminrl(&mut self, b: &NutBasin, nd: &mut NutDay)`
- **Padanan SWAT+:** `nut_nminrl` (`nut_nminrl.f90`), termasuk denitrifikasi sebaris (panggilan `nut_denit` di Fortran sudah dikomentari).
- **Tujuan:** Per lapisan: aliran N antara humus aktif dan stabil, mineralisasi N dan P humus, dekomposisi residu dalam tanah, dan denitrifikasi.
- **Masukan dan keluaran:** parameter `cmn`, `n_act_frac`, `cdn`, `sdnco`, `decr_min`; lapisan (`st`, `fc`, `tmp`, `cbn`, kolam N/P). Keluaran ke `NutDay`: `act_sta_n`, `act_nit_n`, `org_lab_p`, `denit` (kg/ha, dijumlahkan atas lapisan).
- **Cara kerja:** Untuk setiap lapisan k (indeks 0):

(1) Lapisan kendali kk: untuk k = 0 dipakai lapisan kedua (bila ada), selain itu kk = k. Air, kapasitas lapang, dan suhu diambil dari kk. Bila suhu kk ≤ 0 °C, seluruh proses lapisan k (termasuk denitrifikasi) dilewati.

(2) Faktor `sut`, `cdg`, `csf` dari `sut_cdg`.

(3) Aliran humus aktif → stabil (Neitsch et al. 2011, pers. 3:1.2.3), dengan β\_trns = 10^-5 dan fr\_actN = `n_act_frac`:

```latex
N_{trns} = \beta_{trns}\left(orgN_{act}\left(\frac{1}{fr_{actN}} - 1\right) - orgN_{sta}\right)
```

Bila positif, dibatasi `hact.n`; bila negatif, besarnya dibatasi `hsta.n`. Lalu `hsta.n += N_trns`, `hact.n -= N_trns`, masing-masing minimum 10^-6; dicatat di `act_sta_n`.

(4) Mineralisasi humus aktif:

```latex
N_{mina} = \min\left(orgN_{act},\; \beta_{min}\,\gamma_c\,orgN_{act}\right),\qquad P_{mina} = \min\left(orgP_{sta},\; 1.4\,N_{mina}\,\frac{orgP_{sta}}{orgN_{sta} + orgN_{act}}\right)
```

(P\_mina = 0 bila penyebut ≤ 10^-6.) `hact.n -= N_mina` (minimum 10^-6), `no3 += N_mina`, `hsta.p -= P_mina`, `lab += P_mina`; dicatat di `act_nit_n` dan `org_lab_p`.

(5) Dekomposisi residu tanah setiap tanaman di lapisan k dengan rumus yang sama dengan `rsd_decomp` (laju `rsdco_pl · ca · csf`, dijepit \[`decr_min`, 1\]); setiap komponen residu < 10^-10 dinolkan. Hasilnya: `no3 += 0,8 N`, `hact.n += 0,2 N`, `lab += 0,8 P`, `hsta.p += 0,2 P`.

(6) Denitrifikasi (kecuali lapisan biozona septik aktif):

```latex
N_{denit} = \begin{cases} NO_3\left(1 - \exp\left(-\beta_{denit}\,\gamma_{tmp}\,\frac{orgC}{100}\right)\right) & \gamma_{sw} \ge \gamma_{sw,thr} \\ 0 & \text{lainnya} \end{cases}
```

`no3 = max(0,0001, no3 − N_denit)`; dicatat di `denit`.

- **Kebiasaan SWAT+ / catatan port:**
  - Lapisan teratas memakai air dan suhu lapisan kedua (`kk = 2` di Fortran), termasuk untuk denitrifikasinya; dicatat di `docs/swat-quirks.md`.
  - Faktor air denitrifikasi adalah `sut` versi SWAT+ (0,1 + 0,9√(SW/FC)), bukan rasio SW/FC teori 2009; ambang `sdnco` dibandingkan dengan nilai ini.
  - Lapisan biozona septik (`sep_opt` = 1) tidak didenitrifikasi di sini karena `sep_biozone` menanganinya.
  - Lantai 10^-6 dan 0,0001 pada kolam N mengikuti Fortran dan sedikit menciptakan massa; dipertahankan demi bit-for-bit.

#### 9.3.3  `nut_nitvol`

- **Lokasi:** `nutrient.rs`, `pub(super) fn nut_nitvol(&mut self)`
- **Padanan SWAT+:** `nut_nitvol` (`nut_nitvol.f90`).
- **Tujuan:** Nitrifikasi amonium menjadi nitrat dan volatilisasi amonium ke atmosfer, per lapisan (Neitsch et al. 2011, bagian 3:1.3, mengikuti Reddy et al. 1979 dan EPIC).
- **Masukan dan keluaran:** `nh4`, `no3` (kg/ha), `tmp` (°C), `st`, `fc`, `wpmm` (mm), kedalaman `d` (mm). Mengubah `nh4` dan `no3`. N yang tervolatilisasi tidak dicatat di `NutDay`.
- **Cara kerja:** Untuk setiap lapisan, dengan faktor KTK tetap η\_vol,cec = 0,15:

(1) Faktor suhu nitrifikasi/volatilisasi:

```latex
\eta_{tmp} = 0.41\,\frac{T - 5}{10}
```

Proses hanya berjalan bila `nh4` > 0, η\_tmp ≥ 0,001, dan lapisan bukan biozona septik aktif.

(2) Faktor air (setara SW/(0,25 FC), maksimum 1):

```latex
\eta_{sw} = \begin{cases} \frac{SW}{0.25\,FC} & SW < 0.25\,FC \\ 1 & \text{lainnya} \end{cases}
```

Di kode dihitung dari `sw25 = WP + 0,25 FC` dan `swwp = WP + SW`.

(3) Faktor kedalaman volatilisasi dengan z\_mid kedalaman tengah lapisan (mm):

```latex
\eta_{midz} = 1 - \frac{z_{mid}}{z_{mid} + \exp(4.706 - 0.0305\,z_{mid})}
```

(4) Faktor regulasi dan total kehilangan amonium:

```latex
\eta_{nit} = \eta_{tmp}\,\eta_{sw},\quad \eta_{vol} = \eta_{tmp}\,\eta_{midz}\,\eta_{vol,cec},\quad N_{nit|vol} = NH_4\left(1 - \exp(-\eta_{nit} - \eta_{vol})\right)
```

(5) Pembagian antara nitrifikasi dan volatilisasi, bila fr\_nit + fr\_vol > 10^-6:

```latex
fr_{nit} = 1 - e^{-\eta_{nit}},\; fr_{vol} = 1 - e^{-\eta_{vol}},\quad N_{vol} = N_{nit|vol}\frac{fr_{vol}}{fr_{nit} + fr_{vol}},\quad N_{nit} = \max(0, N_{nit|vol} - N_{vol})
```

`nh4 = max(10^-6, nh4 − N_nit)`.

(6) `no3 += N_nit`; `nh4 = max(10^-6, nh4 − N_vol)`.

- **Kebiasaan SWAT+ / catatan port:**
  - Bila fr\_nit + fr\_vol ≤ 10^-6, variabel `rnit` dan `rvol` masih berisi fraksi (bukan massa) dan tetap ditambahkan/dikurangkan sebagai kg/ha, seperti Fortran; dampaknya < 10^-6 kg/ha.
  - Pemeriksaan `nh4 < 0` setelah `max(1e-6, …)` tidak pernah benar; dipertahankan sebagai cermin Fortran.
  - Biozona septik aktif (`sep_opt` = 1) mempertahankan amoniumnya karena `sep_biozone` yang menitrifikasinya.

### 9.4  Keseimbangan P mineral: model standar dan model dinamis

P mineral tanah dipisah menjadi tiga kolam: P labil/larut (`lab`), P mineral aktif (`act`), dan P mineral stabil (`sta`), semuanya kg/ha per lapisan. Pilihan model ditentukan oleh `codes.bsn` `sol_p_model` (disebut juga `soil_p`): 0 memakai `nut_pminrl` dengan PSP tetap dari `parameters.bsn`, 1 memakai `nut_pminrl2` dengan PSP dan rasio stabil-aktif yang berubah menurut tanah (Vadas dan White 2010). Keduanya dipanggil setiap hari setelah `nut_nitvol` untuk kedua model karbon.

Catatan dari `docs/nutrients.md` tentang model P 1, yang berlaku untuk fungsi di subbab ini dan `nutcarb_init`:

- PSP berasal dari liat, karbon organik, dan P labil, dijepit 0,1–0,7, dan dihaluskan harian dengan bobot berjalan 30 hari; rasio stabil-aktif dihaluskan 100 hari (pada inisialisasi dijepit 1–7).
- Aliran labil → aktif melambat dengan bertambahnya hari sejak surplus dimulai, dan kedua aliran menyusut di tanah kering (hingga seperempat laju di bawah seperempat kapasitas lapang).
- Laju aktif → stabil naik dengan kalsium karbonat (`0,0023 CaCO3 + 0,005`, 0,002–0,05). `soil_phys_init` memberi lapisan tanpa CaCO3 nilai 2,8 % (rata-rata 20.000 tanah SSURGO), sehingga `soils.sol` berisi nol, seperti yang ditulis SWAT+ Editor, mendapat nilai itu.
- Lapisan tanpa liat mengambil PSP terakhir pada inisialisasi (variabel tersimpan) dan 0,4 setiap hari.
- Skenario uji **Ps\_solp** (Ps\_coast dengan `soil_p` 1, dibuat `scripts/make-pessel-variants.py`) cocok dengan SWAT+ di semua tabel; tanah sawahnya kaya karbon organik sehingga PSP tertahan di batas bawah 0,1.

`docs/nutrients.md` juga mendokumentasikan beban perkotaan dengan regresi USGS (Driver dan Tasker 1990, `urb_ro` = `usgs_reg`, skenario Ps\_urban); fungsi itu berada di modul perkotaan, bukan di dua file bab ini, sehingga hanya disebut di sini.

#### 9.4.1  `nut_pminrl`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub(super) fn nut_pminrl(&mut self, b: &NutBasin, nd: &mut NutDay)`
- **Padanan SWAT+:** `nut_pminrl` (`nut_pminrl.f90`).
- **Tujuan:** Memindahkan P antara kolam labil dan aktif menuju keseimbangan sorpsi PSP, dan antara aktif dan stabil menuju rasio stabil:aktif 4:1 (Jones et al. 1984, EPIC; Neitsch et al. 2011 bagian 3:2.3).
- **Masukan dan keluaran:** `psp` cekungan; `lab`, `act`, `sta` setiap lapisan. Keluaran `lab_min_p` dan `act_sta_p` di `NutDay` (kg/ha, dijumlahkan).
- **Cara kerja:** Dengan π = PSP dan konstanta laju β\_eqP = `bk` = 0,01, untuk setiap lapisan:

(1) Aliran labil → aktif:

```latex
P_{sol|act} = P_{lab} - P_{act}\,\frac{\pi}{1-\pi};\qquad P_{sol|act} \leftarrow \begin{cases} 0.1\,P_{sol|act} & > 0 \\ 0.6\,P_{sol|act} & < 0 \end{cases},\quad P_{sol|act} \le P_{lab}
```

(2) Aliran aktif → stabil:

```latex
P_{act|sta} = \beta_{eqP}\,(4\,P_{act} - P_{sta});\qquad P_{act|sta} \leftarrow 0.1\,P_{act|sta}\;\text{bila negatif},\quad P_{act|sta} \le P_{act}
```

(3) Pembaruan berurutan, masing-masing dijepit ≥ 0: `sta += roc`; `act = act − roc + rmp1`; `lab -= rmp1`.

(4) `lab_min_p += rmp1`, `act_sta_p += roc`.

- **Kebiasaan SWAT+ / catatan port:** Konstanta `bk` = 0,01 di SWAT+ (komentar Fortran: “was .0006”, nilai di dokumentasi teori SWAT 2009). Batas atas aliran negatif (`min(lab)` pada nilai negatif) tidak membatasi apa-apa; aktif bisa menjadi nol oleh penjepitan.

#### 9.4.2  `nut_pminrl2`

- **Lokasi:** `nutrient.rs`, `pub(super) fn nut_pminrl2(&mut self, nd: &mut NutDay)`
- **Padanan SWAT+:** `nut_pminrl2` (`nut_pminrl2.f90`).
- **Tujuan:** Model P dinamis (Vadas et al. 2006; Vadas dan White 2010): PSP dan rasio stabil-aktif dihitung harian dari sifat tanah dan dihaluskan, laju sorpsi bergantung pada lama ketidakseimbangan, laju aktif–stabil pada CaCO3, dan semuanya dikalikan faktor kebasahan.
- **Masukan dan keluaran:** per lapisan `lab`, `act`, `sta` (kg/ha), `clay`, `cbn`, `cal` (CaCO3, %), `st`, `fc` (mm), `bd`, `thick`; status tersimpan `psp_store`, `ssp_store`, `a_days`, `b_days`. Keluaran `lab_min_p`, `act_sta_p` (dinolkan di awal, lalu dijumlahkan).
- **Cara kerja:** Untuk setiap lapisan:

(1) Lantai 10^-6 pada `lab`, `act`, `sta`. Konsentrasi (mg/kg) `solp = lab / w_conv`, `actpp = act / w_conv`, `stap = sta / w_conv`.

(2) PSP harian: bila `clay` > 0 rumus yang sama dengan inisialisasi (−0,045 ln clay + 0,001 solp − 0,035 C + 0,43), selain itu 0,4; dijepit \[0,1, 0,7\]. Bila `psp_store` > 0 dihaluskan lalu disimpan:

```latex
PSP_t = \frac{29\,PSP_{t-1} + PSP}{30}
```

(3) Ketidakseimbangan `rmp1 = lab − act · π/(1−π)`. Bila ≥ 0 (labil → aktif), dengan n = `a_days` (1 bila 0):

```latex
a = 0.918\,e^{-4.603\,\pi},\quad b = -0.238\ln a - 1.126,\quad k_{a} = \min(0.5, \max(0.1,\; a\,n^{b}))
```

`rmp1 ×= k_a`, `a_days += 1`, `b_days = 0`.

(4) Bila `rmp1` < 0 (aktif → labil), `rmp1 = min(rmp1, act)` dan

```latex
k_{b} = \min(1, \max(0.1,\; (0.79 - 1.08\,\pi)\,e^{-0.29}))
```

`rmp1 ×= k_b`, `a_days = 0`, `b_days += 1`.

(5) Koefisien aktif–stabil dari CaCO3 dan rasio stabil-aktif:

```latex
k_{as} = \min(0.05, \max(0.002,\; 0.0023\,CaCO_3 + 0.005)),\quad SSP = \min(10, \max(0.7,\; 25.044\,(P_{act}^{ppm}(1 + \tfrac{\pi}{1-\pi}))^{-0.3833}))
```

Bila `ssp_store` > 0: `SSP = (SSP + 99 ssp_store)/100`. Aliran:

```latex
P_{act|sta} = k_{as}\left(SSP\,P_{act}\left(1 + \frac{\pi}{1-\pi}\right) - P_{sta}\right)
```

lalu `ssp_store = SSP`.

(6) Faktor kebasahan `w = min(1, max(0,25, SW/FC))`; `rmp1 ×= w`, `roc ×= w`.

(7) Bila `solp + actpp + stap` < 10.000 mg/kg, kolam diperbarui seperti `nut_pminrl` (masing-masing dijepit ≥ 0); bila tidak, kolam tidak berubah tetapi aliran tetap dicatat.

- **Kebiasaan SWAT+ / catatan port:**
  - `exp(-0.29)` dilipat gfortran saat kompilasi dalam presisi ganda lalu dibulatkan ke f32; port meniru dengan `((-0.29_f32) as f64).exp() as f32` agar bit-for-bit.
  - `rmp1.max(-lab)` pada cabang positif tidak berefek (nilai sudah ≥ 0).
  - Fortran menolkan `a_days`/`b_days` pada hari 1 tahun 1; port mengandalkan nilai awal nol.
  - Penghalusan `ssp_store` memakai SSP yang disimpan sesudah penghalusan, sehingga memori 100 hari bersifat rekursif.

### 9.5  Erosi, enrichment ratio, dan N/P yang terbawa sedimen

Erosi harian HRU dihitung dengan MUSLE (Williams 1975) di `erosion`, yang menggabungkan empat subrutin SWAT+ yang dipanggil dari `surface`: `ero_pkq` (debit puncak), `ero_eiusle` (indeks erosivitas USLE), `ero_cfactor` (faktor C), dan `ero_ysed` (hasil sedimen). Nutrien yang terbawa sedimen dihitung kemudian di `sediment_nutrients` (gabungan `pest_enrsb`, `nut_orgn`, `nut_psed`) pada hari dengan limpasan > 0, debit puncak > 10^-6 m3/s, dan presipitasi efektif > 0. HRU lahan basah (sawah/wetland) tidak melewati `erosion` (`surface` tidak dipanggil untuknya), sehingga `qp_cms` tetap 0 dan nutrien sedimen tidak dihitung; yang keluar adalah yang dilepas wetland (`docs/swat-quirks.md`).

Variabel HRU yang dipakai: `tconc` waktu konsentrasi (jam), `km` luas (km2), `area_ha` luas (ha), `usle_mult` hasil kali faktor USLE K·P·LS·CFRG, `rsd_tot.m` residu permukaan (kg/ha), `ab_gr_com.m` biomassa di atas tanah komunitas (kg/ha), `cht_mx` tinggi kanopi maksimum (m), `sno_mm` salju (mm), `erorgn`/`erorgp` enrichment ratio masukan pengguna.

#### 9.5.1  `c_factor`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub(super) fn c_factor(&self, rsd_covco: f32) -> f32`
- **Padanan SWAT+:** `ero_cfactor` (`ero_cfactor.f90`), cabang `cfac` = 1.
- **Tujuan:** Faktor penutupan dan pengelolaan USLE (C) dari residu permukaan dan biomassa di atas tanah.
- **Masukan dan keluaran:** `rsd_covco` (koefisien residu), `rsd_tot.m`, `ab_gr_com.m` (kg/ha), `cht_mx` (m). Keluaran C ∈ \[10^-10, 1\].
- **Cara kerja:**

(1) Faktor residu, dengan residu dalam t/ha (ditambah 1 kg/ha):

```latex
C_{rsd} = \exp\left(-k_{rsd}\,\frac{rsd + 1}{1000}\right)
```

(2) Fraksi penutupan tajuk dari biomassa B (t/ha) dan faktor biomassa dengan tinggi kanopi H (m), dijepit \[10^-10, 1\]:

```latex
f_{cov} = \frac{B}{B + \exp(1.175 - 1.748\,B)},\qquad C_{bio} = 1 - f_{cov}\,\exp(-0.328\,H)
```

(3) `C = max(10^-10, C_rsd · C_bio)`.

- **Kebiasaan SWAT+ / catatan port:** `ero_cfactor` mengeset `bsn_cc%cfac = 1` sebelum mengujinya, sehingga metode faktor C selalu metode residu-biomassa ini, apa pun isi `codes.bsn` (`docs/swat-quirks.md`). Eksponensial memakai `exp_w`, pembungkus yang menjepit argumen ke \[−80, 80\] (argumen < −80 menghasilkan 0) seperti fungsi `Exp` aman di Fortran.

#### 9.5.2  `erosion`

- **Lokasi:** `nutrient.rs`, `pub(super) fn erosion(&mut self, qday: f32, surfq: f32, precip: f32, phh: f32, b: &NutBasin, nd: &mut NutDay)`
- **Padanan SWAT+:** `ero_pkq` (`ero_pkq.f90`), `ero_eiusle` (`ero_eiusle.f90`), `ero_cfactor` (`ero_cfactor.f90`), `ero_ysed` (`ero_ysed.f90`).
- **Tujuan:** Menghitung debit puncak limpasan, energi-intensitas hujan, faktor C, hasil sedimen MUSLE, dan erosi USLE harian.
- **Masukan dan keluaran:** `qday` dan `surfq` limpasan hari itu (mm; pemanggil memberi `max(0, surfq)` dan `surfq`), `precip` presipitasi HRU (mm), `phh` = α\_0.5 fraksi hujan pada setengah jam puncak (dari stasiun, `precip_half_hr`), `sed_det`, `prf`, `rsd_covco`, `fixes`. Keluaran di `NutDay`: `qp_cms` (m3/s), `usle_ei`, `sedyld` (t), `usle` (t/ha); juga `self.usle_cfac`.
- **Cara kerja:**

(1) Debit puncak (bila `qday` > 10^-6). Untuk `sed_det` = 1 (metode rasional termodifikasi SWAT dengan α\_tc):

```latex
\alpha_{tc} = 1 - \exp(2\,t_{conc}\ln(1 - \alpha_{0.5})),\qquad q_{peak} = \frac{\alpha_{tc}\,Q_{surf}}{t_{conc}}\cdot\frac{A_{km^2}}{3.6}
```

Selain itu (rumus dengan faktor laju puncak `prf`, dalam cfs lalu dikonversi):

```latex
q_{peak,cfs} = \frac{prf}{6578.6}\,\frac{A_{ha}\,Q_{surf}}{t_{conc}},\qquad q_{peak} = \frac{q_{peak,cfs}}{35.3}
```

(2) Indeks erosivitas (bila `qday` > 10^-6, `qp_cms` > 10^-6, `precip` > 10^-4), dengan R hujan (mm):

```latex
x_b = -2\ln(1-\alpha_{0.5}),\quad i_{30} = 2\,R\,\alpha_{0.5},\quad i_{pk} = x_b\,R,\quad EI = \frac{R\,(12.1 + 8.9(\log_{10} i_{pk} - 0.4343))\,i_{30}}{1000}
```

Nilai < 10^-4 dinolkan.

(3) Bila `surfq` > 10^-6 dan `qp_cms` > 10^-6: C dari `c_factor`, disimpan di `usle_cfac`; MUSLE:

```latex
sed = (10\,Q_{surf}\,q_{peak}\,A_{ha})^{0.56}\;C\;K\,P\,LS\,CFRG
```

negatif dinolkan.

(4) Koreksi salju (lihat catatan), lalu `sedyld = sed` dan erosi USLE:

```latex
usle = \frac{1.292\,EI\,C\,K\,P\,LS\,CFRG}{11.8}
```

- **Kebiasaan SWAT+ / catatan port:**
  - Uji salju di `ero_ysed` terbalik: bila ada salju (> 0 mm) hanya hasil < 10^-6 yang dinolkan, dan tanpa salju hasil dibagi `exp(0)`, sehingga batas 100 mm dan reduksi eksponensial tidak pernah berlaku. Dengan `--fix-swat` (`b.fixes`), salju > 100 mm menghentikan erosi dan salju lebih sedikit membagi hasil dengan `exp(3 sno / 25,4)`, seperti SWAT 2012 (`docs/swat-fixes.md`).
  - Bila `qday` ≤ 10^-6, `qp_cms` tetap bernilai awal hari (0).
  - Sedimen per langkah subharian (`ero_ovrsed`) dan sedimen dari *runon* ditangani pemanggil, bukan fungsi ini.

#### 9.5.3  `sediment_nutrients`

- **Lokasi:** `nutrient.rs`, `pub(super) fn sediment_nutrients(&mut self, surfq: f32, erorgn: f32, erorgp: f32, orgn: bool, fixes: bool, nd: &mut NutDay, saved: &mut Saved)`
- **Padanan SWAT+:** `pest_enrsb` (`pest_enrsb.f90`, enrichment ratio), `nut_orgn` (`nut_orgn.f90`), `nut_psed` (`nut_psed.f90`).
- **Tujuan:** Menghitung enrichment ratio, lalu N organik dan P (organik, mineral aktif, mineral stabil) yang meninggalkan lapisan atas bersama sedimen.
- **Masukan dan keluaran:** `surfq` (mm), `nd.sedyld` (t), `erorgn`, `erorgp` (enrichment ratio pengguna; dipakai bila > 0,001), `orgn` (benar bila `cswat` 0), `fixes`. Keluaran `nd.enratio`, `nd.sedorgn`, `nd.sedorgp`, `nd.sedminpa`, `nd.sedminps` (kg/ha); `saved.enratio`, `saved.fr_orgp`, `saved.fr_actmin`, `saved.fr_stamin` (status bersama, lihat catatan). Mengurangi `hact.n`, `hsta.n`, `act`, `sta`, `hsta.p` lapisan 1.
- **Cara kerja:**

(1) `sedyld` < 10^-4 dinolkan. Konsentrasi sedimen dan enrichment ratio (Menzel 1980; Neitsch et al. 2011 pers. 4:2.2.3):

```latex
conc_{sed} = \frac{0.1\,sed}{A_{ha}\,Q_{surf} + 10^{-6}},\qquad \varepsilon_{N:sed} = \min(3,\; 0.78\,conc_{sed}^{-0.2468})
```

(0 bila conc\_sed ≤ 10^-6.) Disimpan juga di `saved.enratio`.

(2) N organik (`nut_orgn`, hanya bila `orgn`), dengan orgN = `hsta.n + hact.n` (kg/ha), ρ\_b dan kedalaman d (mm) lapisan 1, ε = `erorgn` bila > 0,001, selain itu enrichment ratio:

```latex
w_1 = 10^4\,\rho_b\,d,\qquad orgN_{surf} = 1000\,\frac{orgN\,\varepsilon}{w_1}\,\frac{sed}{A_{ha}}
```

(setara 0,1 · orgN · ε · sed / (ρ\_b d A), bentuk teori SWAT 2009.) Bila orgN > 10^-6, pengurangan proporsional dari `hact.n` dan `hsta.n`; bagian yang membuat kolam negatif dikembalikan dari `sedorgn` dan kolam dinolkan.

(3) P yang melekat sedimen (`nut_psed`): `P_att = hsta.p + 2·man_p + sta + act`, dengan P pupuk kandang `man_p` = 0 (belum ada kolam pupuk kandang). Bila P\_att > 10^-9, fraksi disimpan:

```latex
fr_{orgp} = \frac{hsta.p}{P_{att}},\quad fr_{actmin} = \frac{P_{sta}}{P_{att}},\quad fr_{stamin} = \frac{P_{act}}{P_{att}}\quad(\text{tertukar, lihat catatan})
```

(4) Total P sedimen dengan ε = `erorgp` bila > 0,001, selain itu enrichment ratio:

```latex
sedP = 1000\,\frac{P_{att}\,\varepsilon}{w_1}\,\frac{sed}{A_{ha}}
```

Bila sedP > 10^-9: `sedorgp = sedP fr_orgp`; `sedminpa = min(act, sedP fr_actmin)`, dikurangkan dari `act`; `sedminps = min(sta, sedP fr_stamin)`, dikurangkan dari `sta`; dan P humus `min(hsta.p, sedorgp · hsta.p/(hsta.p + man_p))` dikurangkan dari `hsta.p` (bila penyebut > 10^-6). Selain itu ketiga keluaran P dinolkan.

- **Kebiasaan SWAT+ / catatan port:**
  - Fraksi P mineral tertukar di `nut_psed`: bagian “aktif” dihitung dari ukuran kolam stabil dan bagian “stabil” dari kolam aktif. Dengan `--fix-swat` masing-masing dari kolamnya sendiri (`docs/swat-quirks.md`, `docs/swat-fixes.md`).
  - Fraksi `fr_orgp`, `fr_actmin`, `fr_stamin` adalah variabel modul di Fortran; port menyimpannya di `Saved` sehingga bila P\_att ≤ 10^-9 nilai dari pemanggilan terakhir (bisa HRU lain) yang dipakai.
  - Faktor `w_1` memakai kedalaman lapisan dalam mm dengan konstanta 10^4 persis seperti Fortran (“kg/ha” menurut komentar Fortran, meski dimensinya tidak konsisten); hasil akhir setara rumus teori.
  - Dengan `cswat` 2, `orgn` salah dan N organik sedimen dihitung oleh `nut_orgnc2` (subbab 9.10). Komentar dokumentasi Rust fungsi ini yang dulu menyebut `nut_orgnc2` “belum diporting” sudah diperbaiki di 1.0.1; kini komentar itu menunjuk ke `carbon.rs`.
  - P yang diambil dari `hsta.p` pada model karbon `cswat` 2 berasal dari kolam yang praktis kosong, karena model itu memakai `cp.hp`/`cp.hs`.

### 9.6  Transport nitrat dan P terlarut

Dua fungsi ini dipanggil setiap hari (juga untuk HRU lahan basah) setelah deposisi atmosfer ditambahkan ke lapisan atas. Keduanya memakai fluks air yang sudah dihitung oleh perkolasi dan aliran lateral: per lapisan `prk` (perkolasi keluar dari lapisan, mm), `flat` (aliran lateral, mm), `ul` (air pada saturasi, mm), dan untuk drainase bawah permukaan `qtile` (mm) di lapisan `ldrain` (indeks berbasis 1; 0 bila tak ada drainase). `anion_excl` adalah fraksi porositas yang tidak dapat dimasuki anion (θ\_e).

#### 9.6.1  `nut_nlch`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub(super) fn nut_nlch(&mut self, surfq: f32, b: &NutBasin, nd: &mut NutDay)`
- **Padanan SWAT+:** `nut_nlch` (`nut_nlch.f90`).
- **Tujuan:** Memindahkan nitrat bersama limpasan permukaan (dari lapisan 1), aliran lateral, aliran drainase, dan perkolasi, lapisan demi lapisan dari atas ke bawah.
- **Masukan dan keluaran:** `surfq` (mm), `nperco`, `nperco_lchtile`; per lapisan `no3` (kg/ha), `prk`, `flat`, `ul` (mm). Keluaran `nd.surqno3`, `nd.latno3` (dijumlahkan atas lapisan), `nd.tileno3`, `nd.percn` (nitrat yang keluar dari dasar profil), kg/ha.
- **Cara kerja:** Dengan `percnlyr` = nitrat perkolasi dari lapisan di atasnya (0 untuk lapisan 1), untuk setiap lapisan:

(1) `no3 += percnlyr`; nilai < 10^-6 dinolkan.

(2) Volume air bergerak w\_mobile = `prk + flat + 1e-10`, ditambah `surfq` untuk lapisan 1 dan `qtile` untuk lapisan drainase. Konsentrasi nitrat air bergerak (Neitsch et al. 2011 pers. 4:2.1.2), dengan eksponen dijepit ≥ −80:

```latex
conc_{NO_3} = \max\left(0,\; \frac{NO_3\left(1 - \exp\left(\frac{-w_{mobile}}{(1-\theta_e)\,SAT}\right)\right)}{w_{mobile}}\right)
```

(3) Lapisan 1: nitrat limpasan

```latex
NO_{3,surf} = \min\left(NO_3,\; \beta_{NO_3}\,conc_{NO_3}\,Q_{surf}\right)
```

lalu `no3 = max(0,0001, no3 − NO3_surf)`.

(4) Lapisan drainase dengan `qtile` > 0: konsentrasi dihitung ulang dari jumlah `ul` dan `no3` lapisan drainase hingga dasar profil,

```latex
conc_{tile} = \max\left(0,\; \frac{\sum NO_3\left(1 - \exp\left(\frac{-q_{tile}}{(1-\theta_e)\sum SAT}\right)\right)}{q_{tile}}\right),\qquad NO_{3,tile} = conc_{tile}\,\beta_{tile}\,q_{tile}
```

Nitrat drainase diambil berurutan dari lapisan drainase ke bawah; kekurangan yang tersisa setelah lapisan terakhir dikurangkan dari `tileno3`. Konsentrasi `conc_tile` kemudian juga dipakai untuk aliran lateral dan perkolasi lapisan ini.

(5) Nitrat lateral `min(no3, conc · flat)`, ditambahkan ke `latno3`; `no3 = max(0,0001, …)`.

(6) Nitrat perkolasi `percnlyr = min(no3, conc · prk)`; `no3 = max(0,0001, …)`. Untuk lapisan terakhir, `percn = percnlyr`.

- **Kebiasaan SWAT+ / catatan port:**
  - Eksponen drainase tidak dijepit −80 (hanya eksponen lapisan yang dijepit), sama dengan Fortran.
  - Lantai 0,0001 kg/ha dipakai setelah setiap pengurangan, sehingga lapisan tidak pernah benar-benar kosong.
  - Untuk HRU wetland/sawah, `nut_nlch` tetap berjalan dan menggantikan nitrat yang dilepas wetland dengan nitrat tanah (`docs/swat-quirks.md`).

#### 9.6.2  `nut_solp`

- **Lokasi:** `nutrient.rs`, `pub(super) fn nut_solp(&mut self, surfq: f32, runon_solp: f32, b: &NutBasin, nd: &mut NutDay)`
- **Padanan SWAT+:** `nut_solp` (`nut_solp.f90`).
- **Tujuan:** P larut dalam limpasan permukaan dari lapisan 1, pencucian P labil ke bawah profil, dan P labil ke drainase.
- **Masukan dan keluaran:** `surfq` (mm), `runon_solp` (kg/ha P larut dari *runon* wetland), `phoskd` (k\_d, m3/Mg), `pperco`; per lapisan `lab`, `st`, `prk`, `bd`. Keluaran `nd.surqsolp`, `nd.lchlabp`, `nd.tilelabp` (kg/ha).
- **Cara kerja:**

(1) Lapisan 1: `lab += runon_solp`, lalu dengan kedalaman lapisan d (mm):

```latex
P_{surf} = \min\left(P_{lab},\; \max\left(0,\; \frac{P_{lab}\,Q_{surf}}{\rho_b\,d\,k_d + 1}\right)\right)
```

dan `lab -= P_surf`.

(2) Untuk setiap lapisan (kecuali lapisan biozona septik), P tercuci dengan perkolasi:

```latex
v = \frac{-w_{perc}}{0.01\,SW + 0.1\,k_{perc}\,\rho_b},\qquad P_{perc} = \min(P_{lab},\; 0.001\,P_{lab}(1 - e^{v}))
```

P itu dikurangkan dan ditambahkan ke lapisan berikutnya (sehingga dapat tercuci lagi pada hari yang sama), atau menjadi `lchlabp` untuk lapisan terakhir.

(3) Pada lapisan drainase, rumus yang sama dengan `qtile` menggantikan `prk`; hasilnya `tilelabp`.

- **Kebiasaan SWAT+ / catatan port:**
  - Bentuk pencucian ini (eksponensial dengan faktor 0,001 dan penyebut 0,01 SW + 0,1 k\_perc ρ\_b) berbeda dari rumus linear dokumentasi teori SWAT 2009 (P\_perc = P\_lab w\_perc / (10 ρ\_b d k\_d,perc) untuk lapisan permukaan); port mengikuti kode SWAT+ apa adanya.
  - Biozona septik dilewati bila HRU memiliki septik, terlepas dari `sep_opt` (berbeda dengan `nut_nminrl`/`nut_nitvol` yang memeriksa `sep_opt` = 1), karena `sep_biozone` memindahkan P-nya.
  - Eksponensial memakai `exp_w`. Untuk wetland, `nut_solp` menggantikan P larut yang dilepas dengan P larut tanah (`docs/swat-quirks.md`).

### 9.7  Pencampuran tanah: biologis akhir tahun, pengolahan tanah `cswat` 0, dan *puddling* sawah

Pencampuran tanah memindahkan sebagian massa setiap lapisan di atas kedalaman pencampuran ke sebuah “campuran”, lalu membagikan campuran itu kembali ke lapisan-lapisan tersebut sebanding dengan tebalnya. Notasi bersama: massa tanah lapisan l (kg/ha)

```latex
m_l = \frac{h_l}{1000}\cdot 10^4\cdot \rho_{b,1}\cdot 1000\cdot\left(1 - \frac{rock_l}{100}\right)
```

(memakai bobot isi lapisan **pertama** untuk semua lapisan, seperti Fortran). Dengan efisiensi e dan kedalaman D (mm), D < 10 diganti 11, dan d\_{l-1} kedalaman dasar lapisan sebelumnya:

```latex
m^{mix}_l = \begin{cases} e\,m_l & d_l \le D \\ e\,m_l\,\frac{D - d_{l-1}}{h_l} & d_l > D > d_{l-1} \\ 0 & \text{lainnya}\end{cases},\qquad f^{dep}_l = \begin{cases} h_l/D \\ (D - d_{l-1})/D \\ 0 \end{cases}
```

fraksi tercampur `fm = m^mix/m`, fraksi tak tercampur `fnm = (m − m^mix)/m`. Setiap besaran X (kg/ha) menjadi `X_l ← fnm_l X_l + f^dep_l Σ_k fm_k X_k`; tekstur (%) dicampur sebagai massa: `%_l ← (100/m_l)(fnm_l %_l m_l/100 + f^dep_l Σ_k fm_k %_k m_k/100)`.

#### 9.7.1  `year_end_mixing`

- **Lokasi:** `crates/swat-engine/src/hru/nutrient.rs`, `pub(super) fn year_end_mixing(&mut self, emix: f32)`
- **Padanan SWAT+:** panggilan `mgt_newtillmix_cswat0` dari `time_control.f90` pada hari terakhir tahun dengan `hru%hyd%biomix`.
- **Tujuan:** Pencampuran biologis tahunan untuk model karbon asli.
- **Masukan dan keluaran:** `emix` = efisiensi pencampuran biologis HRU (`hydrology.hyd` `biomix`, mis. 0,2 pada Ames). Mengubah lapisan di atas 50 mm.
- **Cara kerja:** Bila `emix` ≤ 10^-6 tidak ada yang dilakukan; selain itu `tillmix_cswat0(emix, 0, 0)`.
- **Kebiasaan SWAT+ / catatan port:** Dicatat di `docs/swat-quirks.md` (“Soil mixing at the end of each year”): lapisan di atas 50 mm bertukar bagian air, N, P, dan humus, dan sebagian residu permukaan masuk ke dalamnya. Pemanggil (`hru/mod.rs`) melakukannya hanya untuk `cswat` 0; model karbon memakai pencampuran biologis harian (subbab 9.11).

#### 9.7.2  `tillmix_cswat0`

- **Lokasi:** `nutrient.rs`, `pub(super) fn tillmix_cswat0(&mut self, bmix: f32, effmix: f32, deptil: f32)`
- **Padanan SWAT+:** `mgt_newtillmix_cswat0` (`mgt_newtillmix_cswat0.f90`).
- **Tujuan:** Pencampuran biologis (bila `bmix` > 0) atau pengolahan tanah dengan efisiensi `effmix` hingga `deptil` mm untuk model karbon asli (juga dipakai pada `cswat` 2 tanpa basis karbon di pemanggil).
- **Masukan dan keluaran:** `bmix`, `effmix` (fraksi), `deptil` (mm). Mengubah `no3`, `nh4`, `wsol`, `lab`, `act`, `sta`, `hact`, `hsta`, kolam `cp` (tot, hs, hp, microb, str, lig, meta), residu tanah dan permukaan, `rsd_tot`, `clay`, `silt`, `sand`, `rock` (hanya pengolahan), `st`; untuk pengolahan juga `till_days = 0`, `till_depth`, `till_switch = true`.
- **Cara kerja:**

(1) Mode biologis: e = `bmix`, D = min(kedalaman dasar lapisan terakhir, 50 mm). Mode pengolahan: e = `effmix`, D = `deptil`, dan penanda pengolahan diset. Bila D ≤ 0 selesai.

(2) Massa `m_l` dan pemotongan D < 10 → 11; hitung `m^mix_l`, `fnm_l`, `f^dep_l` seperti di atas.

(3) Kumpulkan campuran: air `st`, batu (pengolahan saja), pasir, debu, liat (sebagai massa), `no3`, `nh4`, `wsol`, `lab`, `act`, `sta`, `cp.tot`, `hact`, `hsta`, `cp.hs`, `cp.hp`, `cp.microb`, `cp.str`, `cp.lig`, `cp.meta`, dan, per tanaman, `fm · residu permukaan`.

(4) Bagikan kembali ke setiap lapisan dengan `fnm X + f^dep Σ`. Untuk residu tanah tanaman: `soil_rsd = fnm soil_rsd + f^dep mix_rsd`, lalu sebagian residu permukaan dipindah eksplisit, `surf = e f^dep rsd`, ditambahkan ke `soil_rsd` dan dikurangkan dari `rsd` dan `rsd_tot`.

- **Kebiasaan SWAT+ / catatan port:**
  - Campuran residu mengambil residu **permukaan** (dengan bobot fm) tanpa mengurangkannya dari permukaan, sementara residu tanah kehilangan bagian e-nya tanpa masuk campuran; ditambah pemindahan eksplisit residu permukaan. Massa residu tidak kekal; perilaku ini identik dengan Fortran (baris `mix_org%rsd` memakai `pl_mass%rsd`).
  - Mode biologis tidak mencampur batu. Kedalaman 50 mm berasal dari revisi Fortran (komentar “it was 300. MJW (rev 412)”).
  - Pecahan struktural non-lignin (`cp.nonlig`) tidak dicampur.

#### 9.7.3  `tillmix_wet`

- **Lokasi:** `nutrient.rs`, `pub(super) fn tillmix_wet(&mut self, emix: f32, deptil: f32, cswat: i32, fixes: bool)`
- **Padanan SWAT+:** `mgt_newtillmix_wet` (`mgt_newtillmix_wet.f90`).
- **Tujuan:** Pengolahan di bawah genangan sawah (*puddling*): nutrien tanah di atas `deptil` dicampur bersama kolom air, dan air genangan menerima bagiannya sebanding dengan kedalamannya.
- **Masukan dan keluaran:** `emix`, `deptil` (mm), `cswat`, `fixes`; kedalaman air genangan `wet.depth` (m) dan kandungan airnya `wet.water` (`no3`, `nh3`, `solp`, `orgn`, `sedp`, kg). Mengubah kolam lapisan, tekstur, dan nutrien air genangan.
- **Cara kerja:**

(1) Kedalaman total `T = 1000 · depth + deptil` (mm). Bila `deptil` ≤ 0 selesai; D < 10 → 11. Massa `m_l` dengan ρ\_b lapisan 1.

(2) Selain `f^dep` (berbasis D), dihitung `f^dep1` berbasis T: `h_l/T` untuk lapisan penuh, `(T − d_{l-1})/T` untuk lapisan terpotong.

(3) Jumlah campuran `smix` per indeks: 1 nitrat, 2 `hsta.n`, 3 amonium, 4 P labil, 5 `hsta.p`, 6 `hact.n`, 7 P aktif, 8 `cp.tot.n`, 9 `cp.tot.p`, 10 P stabil, 11 `cp.tot.m`, 15 `cp.tot.c` (masing-masing × fm); 17–19 liat, debu, pasir × `f^dep`, kemudian dibagi D. Untuk `cswat` 2 juga 12 komponen kolam karbon (C, N, massa serasah struktural, lignin, metabolik, N mikroba, `hact.n`, `hsta.n`). Dengan `fixes`, campuran dimulai dari nutrien air genangan per ha (no3, nh3, solp, orgn, sedp di indeks 1, 3, 4, 8, 9).

(4) Air genangan menerima bagian `1000 depth / T` (kg = kg/ha × luas):

```latex
W_{NO_3} = s_1\frac{1000\,z_w}{T}A,\; W_{NH_3} = s_3\frac{1000\,z_w}{T}A,\; W_{solP} = s_7\frac{1000\,z_w}{T}A,\; W_{orgN} = s_8\frac{1000\,z_w}{T}A,\; W_{sedP} = s_9\frac{1000\,z_w}{T}A
```

(5) Lapisan: nitrat, amonium, P aktif, `cp.tot.n`, `cp.tot.p` memakai `f^dep1`; sisanya `f^dep`. Tekstur: `% = % fnm + smix · frac_mixed`, dengan `frac_mixed` sisa iterasi terakhir (lapisan terbawah). Kolam karbon (`cswat` 2) dengan `f^dep`.

- **Kebiasaan SWAT+ / catatan port:**
  - SWAT+ menambahkan nutrien air genangan ke jumlah campuran lalu menolkan jumlah itu, sehingga nutrien genangan hilang dan air hanya menerima nutrien tanah; dengan `--fix-swat` nutrien genangan ikut tercampur (`docs/swat-fixes.md`).
  - Air genangan menerima P mineral **aktif** (indeks 7) sebagai P larut, serta N dan P organik total model karbon (`docs/swat-quirks.md`).
  - Tekstur dicampur dengan `frac_mixed` lapisan terakhir, variabel loop yang tersisa; untuk lapisan di bawah kedalaman nilainya 0 sehingga tekstur lapisan atas hanya dikali fnm.
  - Untuk lapisan terpotong `f^dep1 = (T − d_{l-1})/T` (bukan `(D − d_{l-1})/T`), mengikuti Fortran.
  - Akumulator kolam karbon memakai field `m` dari array `OrgMass` sebagai skalar; ini hanya teknik penyimpanan port.

### 9.8  Model karbon SWAT-C (`cswat` 2): struktur, aliran N, dan residu (`carbon.rs`)

Model karbon `cswat` 2 adalah implementasi SWAT+ dari SWAT-C (Zhang et al. 2013), yang mengadopsi skema CENTURY (Parton et al. 1987, 1994) melalui EPIC (Izaurralde et al. 2006). Setiap lapisan memiliki kolam `Pools` (field `cp` lapisan, padanan `soil1(j)`), masing-masing `OrgMass` {m, c, n, p} dalam kg/ha:

| Field | Kolam | Keterangan |
| --- | --- | --- |
| `tot` | total organik | `str + meta + hs + hp + microb` |
| `hs` | humus lambat (*slow*) | waktu tinggal puluhan tahun |
| `hp` | humus pasif | waktu tinggal ratusan tahun |
| `microb` | biomassa mikroba (BM) | kolam aktif |
| `str` | serasah struktural | = lignin + non-lignin |
| `lig` | bagian lignin serasah struktural |  |
| `nonlig` | bagian non-lignin |  |
| `meta` | serasah metabolik |  |
| `seq` | C terikat | `hs + hp + microb` |

`LayerRates` (padanan `carbdb`/`org_allo`, dari `carbon_lyr.bsn`, dua baris: 1 = lapisan permukaan, 2 = lapisan bawah) memuat laju harian potensial `hp_rate`, `hs_rate`, `microb_rate`, `meta_rate`, `str_rate`, `microb_top_rate` (1/hari), `hs_hp` (fraksi HS → HP), dan fraksi CO2 yang hilang `a1co2` (serasah/mikroba → BM), `asco2` (HS), `apco2` (HP), `abco2` (BM). Baris dengan `layer` di luar 1–2 diabaikan; bila file tidak ada semua laju 0.

`CarbonBasin` mengumpulkan `carbon.bsn` (dengan nilai bawaan bila tidak ada): `frac_seq` 0,95; `frac_hum_microb` 0,02; `frac_hum_slow` 0,54; `frac_hum_passive` 0,44; `prmt_21` (KOC untuk C mikroba, 1000); `prmt_44` (rasio konsentrasi C mikroba di limpasan/lateral terhadap perkolasi, 0,5); `watf` (fungsi air, 1); `bmix_a/b/c` (3, 0, 0); `till_eff_days` (30); `tillmix_a/b/c` (3, 5, −5,5); `till_consf` (0,10); `bio_consf` (0,15); `rtof` (0,5, fraksi pupuk organik ke residu); `photo_degrade_factor` (0,01); `cnr_cap`/`cnr_ref` (500/25); `cpr_cap`/`cpr_ref` (5000/200); `mathers` (`mathers_int` = 1); serta `idc_till` (`codes.bsn`) dan `sdnco`, `cdn`, `decr_min` (`parameters.bsn`).

#### 9.8.1  `CarbonBasin::new`

- **Lokasi:** `crates/swat-engine/src/hru/carbon.rs`, `pub fn new(project: &Project) -> CarbonBasin`
- **Padanan SWAT+:** pembacaan `carbon.bsn` dan `carbon_lyr.bsn` (modul `carbon_module`, `basin_read_*`), serta `bsn_prm`/`bsn_cc`.
- **Tujuan:** Menyusun parameter model karbon sekali per proyek.
- **Masukan dan keluaran:** tabel `project.carbon`, `project.carbon_lyr`, `project.params`, `project.codes`. Keluaran `CarbonBasin`.
- **Cara kerja:** (1) Fungsi bantu `r(nama, bawaan)` dan `i(nama, bawaan)` membaca baris pertama `carbon.bsn`. (2) Untuk setiap baris `carbon_lyr.bsn` dengan `layer` 1 atau 2, isi `rates[layer − 1]` (kolom yang hilang bernilai 0). (3) Parameter `parameters.bsn` dibaca dengan nilai 0 bila tidak ada.
- **Kebiasaan SWAT+ / catatan port:** `CarbonBasin` hanya dibuat bila `cswat` = 2 (`hru/mod.rs`), dan juga dibuat ulang oleh `nutcarb_stage` untuk `carbon_init`.

#### 9.8.2  `np_flow`

- **Lokasi:** `carbon.rs`, `fn np_flow(c_a: f32, e_a: f32, ce_to_b: f32, c_a_to_b: f32) -> (f32, f32, f32)` (privat)
- **Padanan SWAT+:** `nut_np_flow` (`nut_np_flow.f90`).
- **Tujuan:** Untuk aliran karbon dari kolam A ke kolam B, menghitung N (atau P) yang ikut, dan imobilisasi atau mineralisasi yang diperlukan agar aliran memenuhi rasio C:N target B.
- **Masukan dan keluaran:** `c_a`, `e_a` C dan N kolam A (kg/ha), `ce_to_b` rasio C:N target B, `c_a_to_b` aliran C (kg/ha). Keluaran (aliran N, N terimobilisasi, N termineralisasi).
- **Cara kerja:**

(1) Bila E\_A < 10^-6, C\_{A→B} < 10^-6, atau C\_A < 10^-6, semua nol.

(2) Aliran N sebanding:

```latex
E_{A\to B} = E_A\,\frac{C_{A\to B}}{C_A}
```

(3) Bila C\_{A→B}/E\_{A→B} > (C:E)\_B (aliran miskin N), imobilisasi `imm = C_{A→B}/(C:E)_B − E_{A→B}`; selain itu mineralisasi `mnr = E_{A→B} − C_{A→B}/(C:E)_B` dan `E_{A→B} -= mnr`.

- **Kebiasaan SWAT+ / catatan port:** Fortran menghitung N yang dilepas bersama CO2 (`EfromCO2`) tetapi menambahkan variabel lain yang selalu 0 (`efco2`) ke mineralisasi; port menulis `mnr += 0.0` sebagai cermin (`docs/swat-quirks.md`). Akibatnya N dari respirasi tidak dikembalikan ke tanah.

#### 9.8.3  `cbn_surfrsd_decomp`

- **Lokasi:** `carbon.rs`, `pub(super) fn cbn_surfrsd_decomp(&mut self, cb: &CarbonBasin)`
- **Padanan SWAT+:** `cbn_surfrsd_decomp` (`cbn_surfrsd_decomp.f90`).
- **Tujuan:** Residu permukaan kehilangan sebagian karena fotodegradasi, lalu terdekomposisi menjadi serasah metabolik, struktural, dan lignin lapisan teratas.
- **Masukan dan keluaran:** residu permukaan per tanaman `plants[i].rsd`, `rsd_tot`; lapisan 1 (`st`, `fc`, `tmp`, `cp`); `photo_degrade_factor`, `cnr_*`, `cpr_*`, `decr_min`; fraksi alokasi tanaman di atas tanah `fracs.meta_abg`, `str_abg`, `lig_abg`.
- **Cara kerja:** Untuk setiap tanaman:

(1) Fotodegradasi: `photo = f_photo · rsd` dikurangkan dari `rsd` dan `rsd_tot` (hilang dari sistem), juga pada hari beku.

(2) Bila suhu lapisan 1 ≤ 0 °C, lanjut ke tanaman berikutnya.

(3) Faktor `sut`, `cdg`, `csf` dari air dan suhu lapisan 1 (rumus sama dengan `sut_cdg`), dan faktor rasio nutrien dengan batas dan acuan `carbon.bsn`:

```latex
\gamma_{CN} = \exp\left(-0.693\,\frac{\min(CN_{cap}, C/N) - CN_{ref}}{CN_{ref}}\right),\quad \gamma_{CP} = \exp\left(-0.693\,\frac{\min(CP_{cap}, C/P) - CP_{ref}}{CP_{ref}}\right),\quad ca = \min(\gamma_{CN}, \gamma_{CP}, 1)
```

(4) Laju `decr = min(1, max(decr_min, rsdco_pl · ca · csf))`; `decomp = decr · rsd` dikurangkan dari `rsd` dan `rsd_tot`; komponen residu < 10^-10 dinolkan.

(5) Alokasi ke lapisan 1: `meta += meta_abg · decomp`, `str += str_abg · decomp`, `lig += lig_abg · decomp` (keempat komponen m, c, n, p).

- **Kebiasaan SWAT+ / catatan port:** Berbeda dengan `rsd_decomp` (`cswat` 0), tidak ada N/P yang langsung menjadi mineral; semua masuk ke serasah. Kolam `nonlig` tidak diperbarui di sini (lignin bagian dari struktural).

#### 9.8.4  `cbn_rsd_transfer`

- **Lokasi:** `carbon.rs`, `pub(super) fn cbn_rsd_transfer(&mut self)`
- **Padanan SWAT+:** `cbn_rsd_transfer` (`cbn_rsd_transfer.f90`).
- **Tujuan:** Memindahkan seluruh residu tanaman di dalam tanah (akar mati, residu yang terbenam) ke kolam serasah lapisan yang tidak beku.
- **Masukan dan keluaran:** `plants[i].soil_rsd[k]`, fraksi di bawah tanah `fracs.meta_blg`, `str_blg`, `lig_blg`. Mengubah `cp.meta`, `cp.str`, `cp.lig` dan mengosongkan `soil_rsd`.
- **Cara kerja:** Untuk setiap lapisan dengan suhu > 0 °C dan setiap tanaman: `transfer = 1,0 · soil_rsd`; `soil_rsd -= transfer` (komponen < 10^-10 dinolkan, jadi tepat nol); `meta += meta_blg · transfer`, `str += str_blg · transfer`, `lig += lig_blg · transfer`.
- **Kebiasaan SWAT+ / catatan port:** Pemindahan dengan faktor 1,0 dipertahankan sebagai perkalian (bukan penyalinan) agar identik dengan Fortran.

### 9.9  Transformasi kolam karbon: `cbn_zhang2`

Fungsi ini adalah inti model karbon. Setiap hari, untuk setiap lapisan yang tidak beku dan berair, fungsi ini menghitung laju dekomposisi potensial setiap kolam, menyeimbangkan kebutuhan N aliran-aliran itu terhadap N yang tersedia, lalu memindahkan C, N, dan P antara serasah, mikroba, dan humus, sekaligus menghitung denitrifikasi. Skemanya (Zhang et al. 2013; Izaurralde et al. 2006; Parton et al. 1994): serasah metabolik dan bagian non-lignin serasah struktural → mikroba (BM); lignin → humus lambat (HS, 70 %, sisanya CO2); BM → HS dan humus pasif (HP); HS → BM dan HP; HP → BM. Setiap aliran kehilangan sebagian C sebagai CO2.

#### 9.9.1  `cbn_zhang2`
- **Lokasi:** `crates/swat-engine/src/hru/carbon.rs`, `pub(super) fn cbn_zhang2(&mut self, cb: &CarbonBasin, bm: &Biomix) -> f32`
- **Padanan SWAT+:** `cbn_zhang2` (`cbn_zhang2.f90`), dengan `nut_np_flow` untuk aliran N; denitrifikasi sebaris.
- **Tujuan:** Dekomposisi dan aliran antar-kolam model karbon serta denitrifikasi, lapisan demi lapisan. Mengembalikan total N terdenitrifikasi (kg/ha) yang oleh pemanggil disimpan di `nd.denit`.
- **Masukan dan keluaran:** `CarbonBasin` (laju, `watf`, `idc_till`, `till_eff_days`, `sdnco`, `cdn`); `Biomix` (`tmpf` metode faktor suhu, dan parameter `tn`, `top`, `tx` fungsi beta); status HRU `rsd_tot`, `till_switch`, `till_days`, `till_depth`; per lapisan `st`, `fc`, `wpmm`, `ul`, `por`, `tmp`, `cbn`, `clay`, `sand`, `rock`, `tillagef`, `no3`, `nh4`, `lab`, kolam `cp`. Mengubah `no3`, `nh4`, `lab`, dan semua kolam `cp`.
- **Cara kerja:** Untuk setiap lapisan k (indeks 0), dengan kk = lapisan kedua untuk k = 0 dan kk = k selain itu, dan cf = 0 (laju lapisan permukaan) untuk k = 0, 1 selain itu:

(1) Kedalaman tengah `dmid = (d_kk + d_{kk−1})/2`; massa tanah `m = 10^4 h ρ_b (1 − rock/100)`. Lapisan dilewati bila suhu ≤ 0 atau `st` ≤ 0.

(2) Faktor air `sut`, dengan FC dan WC sebagai air total (`fc + wpmm`, `st + wpmm`). Untuk `watf` = 1: bila WC − WP < 0, `0,1 (st_kk/WP)^2`; selain itu `0,1 + 0,9 √(st/fc)`; dijepit [0,05, 1]. Untuk `watf` = 2 (fungsi air-aerasi), dengan ruang pori kosong `svoid = por (1 − WC/(ul + WP))`:

```latex
x_1 = \begin{cases} 0.4\,WC/WP & WC \le WP \\ 0.4 + 0.6\,\frac{WC - WP}{FC - WP} & WC \le FC \\ 1 & \text{lainnya} \end{cases},\quad x_2 = \begin{cases} 0.2 + 0.8\,\frac{svoid - 0.1}{por - 0.1} & svoid \ge 0.1 \\ 0.2\,svoid/0.1 & \text{lainnya}\end{cases}
```

```latex
sut = (1 + 4(1 - x_1))\,x_1^4\;\cdot\;\left(0.5 + 0.5\,\frac{x_2}{x_2 + e^{-20 x_2}}\right)
```

Nilai `watf` lain memberi `sut` = 0.

(3) Faktor pengolahan `till_eff`. Untuk `idc_till` = 1 (DSSAT), selama pengolahan masih berefek (`till_switch` dan `till_days ≤ till_eff_days`): 1,6 untuk lapisan 1 dan lapisan di atas kedalaman olah, interpolasi `1 + 0,6 (D − d_{k−1})/(d_k − d_{k−1})` untuk lapisan terpotong, dan 1 di bawahnya. Untuk `idc_till` = 3: `1 + tillagef` (dari `tillfactor`). Nilai lain: 1.

(4) Faktor suhu `cdg` menurut `tmpf`:

```latex
cdg = \begin{cases} \frac{T}{T + \exp(5.058 - 0.2504\,T)} \;(T \le 35),\; 1 - 0.04(T - 35)\;(T \le 60),\; 0 & tmpf = 1 \\ \frac{(T - T_n)^{q}(T_x - T)}{(T_{op} - T_n)^{q}(T_x - T_{op})},\; q = \frac{T_n - T_{op}}{T_{op} - T_x} & tmpf = 2 \\ 0.9\,\frac{T}{T + \exp(9.93 - 0.312\,T)} + 0.1 & tmpf = 3 \end{cases}
```

(Untuk `tmpf` = 2, nol di luar (T_n, T_x) dan negatif dijepit 0; `tmpf` lain memberi 0.)

(5) Faktor oksigen dan faktor gabungan:

```latex
ox = 1 - \frac{0.8\,z_{mid}}{z_{mid} + \exp(18.40961 - 0.023683632\,z_{mid})},\qquad cs = \min\left(15,\; \sqrt{cdg\cdot sut}\cdot 0.9\cdot ox\cdot till_{eff}\right)
```

(6) Denitrifikasi seperti model lama: bila `sut ≥ sdnco`, `wdn = no3 (1 − exp(−cdn · cdg · cbn/100))`; `no3 = max(0,0001, no3 − wdn)`; dijumlahkan.

(7) Rasio lignin `rlr = min(0,8, lig.m/(str.m + 10^-5))`. Rasio N:C target untuk lapisan 1, dengan kadar N residu (%) `rsdn = 0,1 (rsd_tot.n + meta.n)/(rsd_tot.c/1000 + 10^-5)`:

```latex
NC_{BM} = \begin{cases} \frac{1}{20.05 - 5.0251\,rsdn} & 0.01 < rsdn \le 2 \\ 0.05 & \text{lainnya} \end{cases},\quad NC_{HS} = \frac{NC_{BM}}{5\,NC_{BM} + 1},\quad NC_{HP} = 0.1,\quad x_{BM} = 1
```

Untuk lapisan bawah, `abco2` lapisan bawah diset `0,17 + 0,0068 sand` dan `x_BM = 0,25 + 0,0075 sand`. Dengan N mineral (ppm) `Nmin = 1000 (no3 + nh4)/(m/1000)`: bila > 7,15 maka NC_BM = 0,33, NC_HS = 0,083, NC_HP = 0,143; selain itu

```latex
NC_{BM} = \frac{1}{15 - 1.678\,N_{min}},\quad NC_{HS} = \frac{1}{20 - 1.119\,N_{min}},\quad NC_{HP} = \frac{1}{10 - 0.42\,N_{min}}
```

(8) Fraksi aliran: `asp = max(0,001, hs_hp − 0,00009 clay)` (HS → HP), `a1 = 1 − a1co2`, `asx = 1 − asco2 − asp` (HS → BM), `apx = 1 − apco2` (HP → BM), `abp = 0,003 + 0,00032 clay` (BM → HP).

(9) Laju C potensial (N sebanding dengan rasio kolamnya), kg/ha/hari:

```latex
LSC = k_{str}\,cs\,e^{-3\,rlr}\,C_{str},\; LSLC = LSC\cdot rlr,\; LSLNC = LSC(1 - rlr),\; LMC = k_{meta}\,cs\,C_{meta},\; BMC = k_{bm}\,cs\,x_{BM}\,C_{BM},\; HSC = k_{hs}\,cs\,C_{HS},\; HPC = k_{hp}\,cs\,C_{HP}
```

k_bm = `microb_top_rate` untuk lapisan 1, `microb_rate` untuk lainnya; LMC ≤ 10^-6 dinolkan.

(10) Neraca N (penutupan `balance`). Kebutuhan N setiap aliran ke kolam tujuan:

```latex
pn_1 = LSLNC\,a_1 NC_{BM},\; pn_2 = 0.7\,LSLC\,NC_{HS},\; pn_3 = LMC\,a_1 NC_{BM},\; pn_5 = BMC\,abp\,NC_{HP},\; pn_6 = BMC(1 - abp - abco2)NC_{HS},\; pn_7 = HSC\,asx\,NC_{BM},\; pn_8 = HSC\,asp\,NC_{HP},\; pn_9 = HPC\,apx\,NC_{BM}
```

Untuk lima sumber (struktural: pn1+pn2 terhadap LSN; metabolik: pn3 terhadap LMN; BM: pn5+pn6 terhadap BMN; HS: pn7+pn8 terhadap HSN; HP: pn9 terhadap HPN) dihitung kekurangan c_i dan kelebihan s_i; `sum = Σ s_i`, `trnn = Σ c_i`.

(11) Pembatasan N: `wmin = max(10^-5, no3 + nh4 + sum)`; `reduc = wmin/trnn` bila wmin < trnn, selain itu 1. Aliran (C dan N) dari sumber yang kekurangan (c_i > 0) dikalikan `reduc`. Neraca dihitung ulang dengan aliran aktual. Bila `rnmn = sum − trnn` > 0, nitrat yang melebihi `rnmn` dipindah ke amonium (`min_n = max(0, no3 − rnmn)`; `nh4 += min_n`; `no3 -= min_n`).

(12) P dari mineralisasi humus dan dekomposisi serasah:

```latex
P_{hum} = \min\left(P_{HP},\; 1.4\,\frac{HSN_a + HPN_a}{N_{HS} + N_{HP} + 10^{-6}}\,P_{HP}\right),\qquad decr = \min\left(1, \frac{LSC_a + LMC_a}{C_{str} + C_{meta} + 10^{-6}}\right),\; P_{rsd} = decr\cdot P_{tot}
```

`hp.p -= P_hum`, `lab += P_hum`; `tot.p -= P_rsd`, `lab += 0,8 P_rsd`, `hp.p += 0,2 P_rsd`.

(13) Aliran aktual dibatasi isi kolam (`LSC ≤ C_str`, `LSLC ≤ C_lig`, `LMC ≤ C_meta`). CO2 dari serasah struktural, `0,3 LSLC + a1co2 LSLNC`, dihitung tetapi tidak dipakai.

(14) Delapan aliran N dihitung dengan `np_flow` (sumber, target C:N): HP→BM (`apx HPC`, 1/NC_BM), HS→BM (`asx HSC`, 1/NC_BM), metabolik→BM (`a1 LMC`, 1/NC_BM), struktural→BM (`a1 LSLNC`, 1/NC_BM), struktural→HS (`0,7 LSLC`, 1/NC_HS), BM→HS (`BMC (1 − abp − abco2)`, 1/NC_HS), BM→HP (`BMC abp`, 1/NC_HS), HS→HP (`HSC asp`, 1/NC_HP).

(15) Pembaruan C:

```latex
C_{BM} \mathrel{+}= -BMC + apx\,HPC + asx\,HSC + a_1(LMC + LSLNC),\quad C_{HS} \mathrel{+}= -HSC + 0.7\,LSLC + BMC(1 - abp - abco2),\quad C_{HP} \mathrel{+}= -HPC + asp\,HSC + abp\,BMC
```

Lignin dan non-lignin dikurangi alirannya (minimum 10^-10), lalu `str.c = nonlig.c + lig.c`. Massa berkurang sebesar C/0,42 (minimum 10^-10). Untuk metabolik (bila `meta.m` > 0): `meta.m -= LMC / max(0,42, meta.c/meta.m)`, `meta.c -= LMC`. `no3` dan `nh4` negatif dinolkan.

(16) Pembaruan N: `meta.n` dan `str.n` dikurangi aliran keluar dan mineralisasinya (minimum 0,001); `microb.n`, `hs.n`, `hp.n` menerima aliran masuk dan imobilisasi, dikurangi aliran keluar dan mineralisasinya. Terakhir `tot.c = str.c + meta.c + hp.c + hs.c + microb.c` dan `seq.c = hp.c + hs.c + microb.c`.

- **Kebiasaan SWAT+ / catatan port:**
  - N termineralisasi tidak pernah sampai ke kolam mineral. `cbn_zhang2` memindahkan N antara serasah, mikroba, dan humus serta mengurangkan bagian termineralisasi dari kolam sumber, tetapi tidak menambahkannya ke nitrat atau amonium. N mineral hanya berubah karena denitrifikasi dan pemindahan nitrat ke amonium pada langkah (11) (`docs/swat-quirks.md`).
  - Rasio N:C mikroba lapisan 1 mengabaikan N residu di atas 2 %: uji pertama diikuti `if/else` yang `else`-nya menimpa dengan 0,05.
  - Dekomposisi lapisan 1 memakai kedalaman lapisan kedua untuk faktor oksigennya, dan air lapisan kedua pada cabang kering `watf` 1. Lapisan di bawah lapisan 1 mengeset `org_allo(2)%abco2` (variabel modul bersama) dari pasirnya sendiri. Port menyalin `cb.rates` ke `allo` setiap panggilan; karena setiap lapisan bawah mengeset nilainya sebelum dipakai, hasilnya sama.
  - Aliran BM → HP memakai target C:N 1/NC_HS, bukan 1/NC_HP, persis seperti kode SWAT+.
  - Variabel `co2fstr` dihitung lalu dibuang (`let _ = co2fstr`) untuk mempertahankan urutan operasi Fortran.
  - Fungsi lokal `balance` (penutupan Rust) menggantikan blok kode yang ditulis dua kali di Fortran; parameter `ls_c` tidak dipakai di dalamnya, sesuai Fortran.

### 9.10  Kehilangan N organik dan karbon bersama air dan sedimen: `nut_orgnc2`

#### 9.10.1  `nut_orgnc2`
- **Lokasi:** `crates/swat-engine/src/hru/carbon.rs`, `pub(super) fn nut_orgnc2(&mut self, surfq: f32, sedyld: f32, enratio: f32, cb: &CarbonBasin, saved: &mut Saved) -> f32`
- **Padanan SWAT+:** `nut_orgnc2` (`nut_orgnc2.f90`).
- **Tujuan:** Untuk model karbon `cswat` 2, dipanggil pada hari dengan limpasan, debit puncak, dan presipitasi (setelah `sediment_nutrients`). N organik meninggalkan lapisan atas bersama sedimen, karbon humus dan residu tererosi, dan karbon mikroba terbawa air (limpasan, lateral, perkolasi) lalu turun di profil. Mengembalikan N organik sedimen (kg/ha), yang oleh pemanggil disimpan di `nd.sedorgn`.
- **Masukan dan keluaran:** `surfq` (mm), `sedyld` (t), `enratio`, `erorgn` HRU, `prmt_21` (KOC), `prmt_44`; lapisan 1 (`bd`, `d`, `rock`, `ul`, `prk`, `flat`), lapisan bawah (`por`, `thick`, `wpmm`, `prk`, `flat`); kolam `cp`; residu tanaman; `saved.c_microb_sed` (status bersama).
- **Cara kerja:**

(1) N organik lapisan 1 termasuk N residu permukaan: `orgN = hp.n + hs.n + rsd_tot.n`. Dengan `w_1 = ρ_b d/100` dan ε = `erorgn` bila > 0,001, selain itu `enratio`:

```latex
conc = \frac{orgN\,\varepsilon}{w_1},\qquad sedorgN = 0.001\,conc\,\frac{sed}{A_{ha}}
```

(setara dengan rumus `nut_orgn`, 0,1 · orgN · ε · sed/(ρ_b d A)).

(2) Bila orgN > 10^-6, fraksi tersisa `r = 1 − sedorgN/orgN` dikalikan ke `tot.n`, `hs.n`, `hp.n`, N residu permukaan, `meta.n`, `str.n`, `lig.n` (bukan `microb.n`).

(3) Erosi karbon. Massa tanah lapisan 1 `m = (d/1000)·10^4·ρ_b·1000·(1 − rock/100)`; `tot.c = hp.c + hs.c + meta.c + str.c`;

```latex
fr_{ero} = \min\left(0.9,\; \frac{sed/A_{ha}}{m/1000}\right)
```

`tot.c`, `hs.c`, `hp.c`, dan C residu permukaan setiap tanaman dikalikan `1 − fr_ero`.

(4) Karbon mikroba terbawa air (bila `microb.c` > 0,01): `tot.c` dihitung ulang termasuk mikroba, lalu

```latex
f_{koc} = 10^{-4}\,K_{OC}\,C_{tot},\quad Q_{tot} = Q_{surf} + w_{perc} + Q_{lat},\quad C_{loss} = C_{BM}\left(1 - \exp\left(\frac{-Q_{tot}}{SAT + f_{koc}}\right)\right)
```

(bila Q_tot > 10^-10). Konsentrasi vertikal `C_loss/(w_perc + prmt_44 (Q_surf + Q_lat))` dikali `w_perc` memberi C yang turun ke lapisan 2 (`c_vert`); `microb.c -= C_loss`. Bila `fr_ero` > 0, C mikroba bersama sedimen `saved.c_microb_sed = fr_ero · f_koc · microb.c/(SAT + f_koc)`.

(5) `microb.c -= saved.c_microb_sed` (selalu), lalu `tot.c` dihitung ulang.

(6) Lapisan bawah, berurutan: `C_perc = microb.c + c_vert`; bila ≥ 0,01 dan `prk + flat` > 0, `c_vert = C_perc (1 − exp(−(prk + flat)/(por · thick − wpmm)))`, selain itu 0; `microb.c = C_perc − c_vert`; `tot.c` dan `seq.c` dihitung ulang. `c_vert` dari lapisan terakhir keluar dari profil.

- **Kebiasaan SWAT+ / catatan port:**
  - N organik dan karbon hanya hilang pada hari dengan sedimen, karena fungsi ini (yang juga memindahkan karbon mikroba ke bawah bersama perkolasi) hanya dipanggil bila ada limpasan, debit puncak, dan presipitasi (`docs/swat-quirks.md`).
  - `c_microb_sed` adalah variabel modul: nilainya tetap dari terakhir kali dihitung, untuk HRU mana pun, dan dikurangkan lagi setiap kali fungsi ini berjalan. Port menyimpannya di `Saved`.
  - N residu yang dihapus adalah milik tanaman 1, dikalikan `r` sebanyak jumlah tanaman (loop atas tanaman selalu menskalakan tanaman 1), dan `rsd_tot` tidak diperbarui (`docs/swat-quirks.md`).
  - Erosi karbon tidak mengurangi C serasah metabolik dan struktural maupun C mikroba (kecuali lewat `c_microb_sed`); `tot.c` yang dikurangi kemudian dihitung ulang dari kolam, sehingga pengurangan `tot.c` pada langkah (3) tidak bertahan.
  - Konstanta `w_1` di sini (`ρ_b d/100`) berbeda bentuk dengan `nut_orgn` (`10^4 ρ_b d`), tetapi hasil akhirnya setara.

### 9.11  Pencampuran biologis harian dan pengolahan tanah pada model karbon

Dengan `cswat` 2, pencampuran biologis berjalan setiap hari (`Hru::biomix` di `hru/mod.rs`, padanan `mgt_biomix.f90`). Pemanggil menghitung efisiensi per lapisan (pulih sebesar `bio_consf` dari nilai awalnya per hari setelah pengolahan, nol pada hari pengolahan), dikalikan faktor suhu (`tmpf` 3: faktor SWAT; lainnya fungsi beta `fcgd`), lalu rata-rata tertimbang tebal `avg_emix` hingga kedalaman pencampuran (berhenti pada lapisan beku), massa lapisan, `fnm`, `f^dep`, `fm`, serta mencampur air `st`. Bagian nutrien, kolam, residu, dan tekstur dikerjakan `biomix_pools`, yang kemudian memanggil `tillfactor`. Pengolahan tanah pada `cswat` 2 memakai `tillmix_cswat1`.

#### 9.11.1  `biomix_pools`
- **Lokasi:** `crates/swat-engine/src/hru/carbon.rs`, `pub(super) fn biomix_pools(&mut self, frac_mixed: &[f32], frac_non_mixed: &[f32], frac_dep: &[f32], sol_mass: &[f32], avg_emix: f32, lyr_exit: usize, dtil: f32, cb: &CarbonBasin)`
- **Padanan SWAT+:** bagian nutrien dan kolam `mgt_biomix` (`mgt_biomix.f90`), lalu `mgt_tillfactor`.
- **Tujuan:** Mencampur N dan P mineral, kolam karbon, residu, dan tekstur lapisan-lapisan di atas kedalaman pencampuran biologis.
- **Masukan dan keluaran:** fraksi per lapisan dari pemanggil (`frac_mixed` = fm, `frac_non_mixed` = fnm, `frac_dep`), massa `sol_mass` (kg/ha), `avg_emix`, `lyr_exit` (lapisan pertama yang tidak tercampur), `dtil` (mm). Mengubah `no3`, `nh4`, `wsol`, `lab`, `act`, `sta`, `cp` (tot, hs, hp, microb, str, lig, meta), residu tanah dan permukaan, `rsd_tot`, `clay`, `silt`, `sand`, lalu faktor pengolahan lewat `tillfactor`.
- **Cara kerja:**

(1) Untuk lapisan l < min(`lyr_exit`, panjang `frac_mixed`), jumlahkan `fm_l · X_l` untuk pasir, debu, liat (sebagai massa `% · m/100`), `no3`, `nh4`, `wsol`, `lab`, `act`, `sta`, dan kolam `cp`. Residu per tanaman: `mix_rsd += f^dep_l · avg_emix · (soil_rsd_l + rsd_permukaan)`.

(2) Untuk l < `lyr_exit`: `X_l = fnm_l X_l + f^dep_l Σ`; `soil_rsd_l = fnm_l soil_rsd_l + f^dep_l mix_rsd`; residu permukaan `surf = f^dep_l · avg_emix · rsd` dikurangkan dari `rsd` dan `rsd_tot`; tekstur dicampur sebagai massa.

(3) `tillfactor(true, avg_emix, dtil, cb, till_days)`.

- **Kebiasaan SWAT+ / catatan port:**
  - Pencampuran biologis mencampur tekstur dan kolam setiap hari: liat, debu, pasir, N dan P mineral, humus, mikroba, dan serasah lapisan atas yang tidak beku, tetapi tidak bagian non-lignin serasah struktural (`nonlig`), yang mempertahankan nilainya sendiri. Residu permukaan yang masuk ke campuran dihitung sekali per lapisan tercampur (`docs/swat-quirks.md`).
  - Kolam humus model lama (`hact`, `hsta`) di Fortran ikut dicampur, tetapi kosong pada `cswat` 2; port tidak mencampurnya (komentar di kode).
  - Air (`st`) dicampur oleh pemanggil sebelum fungsi ini; batu tidak dicampur.

#### 9.11.2  `tillfactor`
- **Lokasi:** `carbon.rs`, `pub(super) fn tillfactor(&mut self, bio_mix_event: bool, emix: f32, dtil: f32, cb: &CarbonBasin, till_days: i32)`
- **Padanan SWAT+:** `mgt_tillfactor` (`mgt_tillfactor.f90`).
- **Tujuan:** Menghitung efek pencampuran (biologis atau pengolahan) terhadap laju dekomposisi setiap lapisan yang tidak beku, `tillagef`, yang dipakai `cbn_zhang2` bila `idc_till` = 3.
- **Masukan dan keluaran:** `bio_mix_event`, `emix` (efisiensi), `dtil` (mm), `bmix_a/b/c`, `tillmix_a/b/c`, `till_consf`, `till_days`; per lapisan `clay`, `tmp`, `st`, `fc`, `tillagef_tillmix`, `tillagef_biomix`. Keluaran `tillagef_tillmix`, `tillagef_biomix`, `tillagef`.
- **Cara kerja:** Untuk setiap lapisan:

(1) Lapisan penuh di atas D: `frac_mixed = 1`. Lapisan yang memuat batas D: `emix ← emix (D − d_{l−1})/h_l` dan `frac_mixed = 1 − (D − d_{l−1})/h_l`. Di bawahnya: `frac_mixed = 0` dan `emix = 0` (dan tetap 0 untuk lapisan berikutnya).

(2) Koefisien maksimum: untuk pencampuran biologis `zz = bmix_a + bmix_b exp(bmix_c · clay/100)` dan `yy = 0`; untuk pengolahan `zz = tillmix_a + tillmix_b exp(tillmix_c · clay/100)` dan `yy = tillagef_tillmix/zz` (efek pengolahan sebelumnya).

(3) Bila `yy` > 0,01, efisiensi setara dari efek sebelumnya:

```latex
x = y^{\exp(-0.13 + 1.06\,y)}\cdot\min\left(10,\; \exp(0.64 + 0.64\,y^{10})\right)
```

selain itu x = 0. `csdr = x + emix`.

(4) Bila lapisan beku dan ini pencampuran biologis: `tillagef_biomix = 0`. Bila lapisan tidak beku:

```latex
f = zz\,\frac{csdr}{csdr + \exp(1 - 2\,csdr)}
```

untuk pencampuran biologis `tillagef_biomix = f`; untuk pengolahan `tillagef_tillmix = f · frac_mixed` (bila positif).

(5) Bila `till_days` > 0 (lapisan tidak beku), efek pengolahan meluruh karena konsolidasi: `consf = till_consf` bila `st ≥ fc`, selain itu `till_consf · st/fc`; `tillagef_tillmix *= 1 − consf`, dan ≤ 0,01 dinolkan. Lalu `tillagef = tillagef_tillmix + tillagef_biomix`.

- **Kebiasaan SWAT+ / catatan port:**
  - Pada lapisan tempat kedalaman berakhir, efek pengolahan diboboti bagian lapisan yang berada di **bawah** kedalaman (`1 − (D − d_{l−1})/h_l`) (`docs/swat-quirks.md`).
  - Efisiensi yang sudah direduksi untuk lapisan batas tetap 0 untuk lapisan di bawahnya (variabel `emix` diubah di dalam loop).
  - Selama `till_eff_days` (100 pada Ames), faktor meluruh setiap kali fungsi ini dipanggil, yang setelah pengolahan berarti setiap hari pencampuran biologis berjalan; pencampuran biologis berhenti pada hari pengolahan dan pulih sebesar `bio_consf` dari nilai penuhnya per hari.
  - Pangkat `y^10` dihitung dengan `crate::fortran::rpow`, peniru pangkat riil gfortran, agar bit-for-bit; eksponensial memakai `exp_w`.

#### 9.11.3  `tillmix_cswat1`
- **Lokasi:** `carbon.rs`, `pub(super) fn tillmix_cswat1(&mut self, effmix: f32, deptil: f32, cb: &CarbonBasin)`
- **Padanan SWAT+:** `mgt_newtillmix_cswat1` (`mgt_newtillmix_cswat1.f90`).
- **Tujuan:** Pengolahan tanah dengan efisiensi `effmix` hingga `deptil` mm pada model karbon. Lapisan di atas kedalaman menyerahkan sebagian tanah (liat, debu, pasir, batu), air, N dan P mineral, humus, dan kolam karbon ke sebuah campuran yang dibagikan kembali sebanding tebal; lalu `mgt_tillfactor`.
- **Masukan dan keluaran:** `effmix` (fraksi), `deptil` (mm), `CarbonBasin`. Mengeset `till_days = 0`, `till_depth`, `till_switch = true`; mengubah kolam, residu, tekstur, air, dan faktor pengolahan.
- **Cara kerja:**

(1) Penanda pengolahan diset (bahkan bila kedalaman ≤ 10^-6, yang kemudian mengakhiri fungsi). Massa `m_l` memakai ρ_b lapisan 1; D < 10 → 11.

(2) Untuk setiap lapisan sampai lapisan pertama di bawah D: hitung `m^mix`, `fnm`, `f^dep` (rumus subbab 9.7). Lapisan pertama di bawah D mendapat `tillagef = 0`, menjadi `lyr_exit`, dan loop berhenti. Kumpulkan `fm · X` untuk `st`, batu, pasir, debu, liat (sebagai massa), `no3`, `nh4`, `wsol`, `lab`, `act`, `sta`, `cp.tot`, `hact`, `hsta`, dan kolam `cp`. Residu per tanaman: `mix_rsd += f^dep · e · soil_rsd_l + f^dep · e · rsd_permukaan`.

(3) Untuk l < `lyr_exit`: `X = fnm X + f^dep Σ`; `soil_rsd = fnm soil_rsd + f^dep mix_rsd`; `surf = f^dep · e · rsd` dikurangkan dari `rsd` (yang tersisa) dan `rsd_tot`; tekstur dan batu dicampur sebagai massa; `st = fnm st + f^dep Σst`.

(4) `tillfactor(false, e, D, cb, till_days)`.

- **Kebiasaan SWAT+ / catatan port:**
  - Massa setiap lapisan dihitung dengan bobot isi lapisan pertama (`docs/swat-quirks.md`).
  - Residu permukaan masuk ke campuran sekali per lapisan tercampur, tetapi keluar dari permukaan lapisan demi lapisan, setiap kali sebagai bagian dari sisa: yang keluar lebih sedikit daripada yang masuk, sehingga massa residu bertambah (`docs/swat-quirks.md`).
  - `nonlig` tidak dicampur, sama dengan `biomix_pools`.
  - Pemanggil (`hru/mgt.rs`) memakai fungsi ini untuk operasi `till` dan pengolahan lain bila `cswat` = 2 dan basis karbon ada; selain itu `tillmix_cswat0`. Di bawah genangan sawah dipakai `tillmix_wet`.

### 9.12  Ringkasan cakupan fungsi

| File | Fungsi (jumlah) |
|---|---|
| `hru/nutrient.rs` | `NutBasin::new`, `SoilTest::default`, `sut_cdg`, `residue_ca`, `nutcarb_init`, `rsd_decomp`, `nut_nminrl`, `nut_nitvol`, `nut_pminrl2`, `nut_pminrl`, `c_factor`, `erosion`, `sediment_nutrients`, `nut_nlch`, `nut_solp`, `year_end_mixing`, `tillmix_cswat0`, `tillmix_wet` (18) |
| `hru/carbon.rs` | `CarbonBasin::new`, `np_flow`, `carbon_init`, `cbn_surfrsd_decomp`, `cbn_rsd_transfer`, `cbn_zhang2` (dengan penutupan lokal `balance`), `nut_orgnc2`, `biomix_pools`, `tillfactor`, `tillmix_cswat1` (10) |
