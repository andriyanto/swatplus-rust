## Bab 13  Reservoir, danau, dan aturan pelepasan

Bab ini membahas objek *reservoir* (waduk dan danau buatan maupun alami) dan objek *recall* (beban titik dan aliran terukur yang dimasukkan ke jaringan), serta rutinitas bersama untuk pengendapan sedimen dan nutrien di badan air. Dua file Rust dicakup seluruhnya: `crates/swat-engine/src/res.rs` (inisialisasi reservoir, tabel keputusan pelepasan `res_rel.dtl`, neraca air harian, dan objek recall) dan `crates/swat-engine/src/waterbody.rs` (parameter `sediment.res`/`nutrients.res`, `res_sediment`, `res_nutrient`, yang juga dipakai lahan basah HRU di `hru/wetland.rs`). Rujukan dokumentasi repo adalah `docs/reservoirs.md` (fase 4, validasi Bh\_res) dan bagian "More reservoir releases" di `docs/engine1.md` (opsi `dyrt1`, `inflo_targ`, `natlake`, `nonirr-h06`, `hydrop`, validasi Bh\_rel).

Padanan Fortran-nya adalah `res_read.f90`, `res_read_hyd.f90`, `res_read_sed.f90`, `res_read_nut.f90`, `res_initial.f90`, `res_control.f90`, `res_hydro.f90`, `conditions.f90` (cabang `vol` dan `month`), `res_sediment.f90`, `res_nutrient.f90`, `theta.f90`, `res_convert_mass` di `hydrograph_module.f90`, `recall_read.f90`, dan cabang `recall` serta `res` di `command.f90`. Dalam alur simulasi, reservoir dan recall adalah objek ruang (*spatial object*) yang dijalankan oleh perintah perutean harian (`route.rs`, Bab perutean): ketika urutan perintah sampai di objek `res`, hidrograf masuk hari itu (`hin`) diserahkan ke `Reservoir::control`, yang mengembalikan hidrograf keluar; objek `recall` cukup mengambil beban hari itu dari tabel yang sudah dibaca saat inisialisasi. Konstruksi kedua objek terjadi di `route.rs` (sekitar baris 507–526) setelah jaringan objek dibangun; bila basin memakai *lapse rate* (`codes.lapse == 1`) dan ada reservoir, port menolak menjalankan.

Semua aritmetika memakai `f32` (setara `real` Fortran) dalam urutan yang sama dengan Fortran agar hasil identik bit demi bit, dan pangkat riil memakai `fortran::rpow` (pembungkus `powf` yang menyembunyikan eksponen dari optimizer agar tidak diganti `sqrt`). Jumlah fungsi yang didokumentasikan: 7 di `res.rs` (`row`, `Reservoir::new`, `Reservoir::operational`, `Reservoir::release`, `Reservoir::control`, `Recall::new`, `Recall::day`) dan 5 di `waterbody.rs` (`find`, `WbPrm::new`, `theta`, `res_sediment`, `res_nutrient`).

### 13.1  Tipe data reservoir dan tabel keputusan (`res.rs`)

Modul `res.rs` menerjemahkan tabel keputusan (*decision table*) dari `res_rel.dtl` satu kali saat inisialisasi menjadi enum Rust yang tertutup, sehingga setiap hari hanya evaluasi numerik yang tersisa. Ini berbeda dari Fortran, yang setiap hari menafsirkan ulang string `lim_var`, `lim_op`, `option`, dan `file_pointer` melalui `select case`. Konsekuensinya, opsi atau operator yang tidak dikenal ditolak di awal (`Err`) alih-alih diam-diam menghasilkan nilai tak terdefinisi.

**Enum `VolTarg`** (privat): volume acuan sebuah kondisi `vol`.

| Varian | `lim_var` di `res_rel.dtl` | Arti |
| --- | --- | --- |
| `Pvol` | `pvol` | volume pada pelimpah utama (*principal spillway*), m3 |
| `Evol` | `evol` | volume pada pelimpah darurat (*emergency spillway*), m3 |
| `EmPv` | `e-pv` | volume utama ditambah sebagian tampungan banjir: pvol + (evol − pvol)·lim |

**Enum `Cond`** (privat): satu kondisi tabel. `Vol { targ, op, lim, alt }` membandingkan volume air reservoir saat ini dengan volume acuan yang dihitung dari `targ`, operator `op` (`*`, `+`, `-`, `/`) dan konstanta `lim`; `Month { lim, alt }` membandingkan bulan kalender dengan `lim` (dibulatkan ke `i32`). `alt` adalah vektor operator perbandingan per alternatif (`<`, `>`, `<=`, `>=`, `=`, `/=`, atau `-` untuk "tidak diuji").

**Enum `Base`** (privat): volume dasar aksi `days` dan `dyrt`, dari `file_pointer` aksi: `Zero` (`null`), `Pvol`, `Evol`.

**Enum `Release`** (privat): cara sebuah aksi melepaskan air (padanan cabang `select case (option)` di `res_hydro`).

| Varian | Opsi `res_rel.dtl` | Parameter (dari `const`, `const2`) | Ringkas |
| --- | --- | --- | --- |
| `Rate(r)` | `rate` | r, m3/s | laju tetap |
| `RatePct(p)` | `rate_pct` | p, % pvol per hari | persentase volume utama |
| `InfloRate{min, plus}` | `inflo_rate` | min, plus dalam m3/s | inflow + tambahan, minimal `min` |
| `InfloFrac(f)` | `inflo_frac` | f, fraksi | fraksi inflow |
| `AbEmer` | `ab_emer` | – | semua di atas evol |
| `Days{days, base, frac}` | `days` | hari, basis, fraksi basis | kuras di atas basis dalam `days` hari |
| `Dyrt{days, base, pct}` | `dyrt` | hari, basis, % pvol | seperti `days` plus laju dasar |
| `Dyrt1{days, pct, op, lim}` | `dyrt1` | hari, % pvol, operator dan limit kondisi rujukan | basis dari kondisi ke-5/4/3 |
| `InfloTarg{lim}` | `inflo_targ` | limit kondisi bernomor `const` | inflow + kelebihan di atas lim·pvol |
| `NatLake{so_frac, kr}` | `natlake` | fraksi tampungan mati, koefisien 1/hari | danau alami (Döll dkk. 2003) |
| `NonIrrH06{alpha}` | `nonirr-h06` | alpha | Hanasaki dkk. (2006), non-irigasi |
| `Hydrop{a, b}` | `hydrop` | amplitudo, fase | skema PLTA HYPE |

Opsi `irr-h06` ditolak (Fortran menghasilkan NaN karena membagi rata-rata kebutuhan yang tak pernah dihitung, nol, dengan dirinya sendiri); `weir` dan `meas` juga ditolak (jatuh ke cabang "option tidak dikenal" di `Reservoir::new`), demikian pula tabel kondisi `ctbl_`.

**Struct `Reservoir`** (publik): data dan keadaan satu reservoir.

| Field | Satuan | Padanan Fortran | Keterangan |
| --- | --- | --- | --- |
| `ob` | indeks | `res_ob%ob` | indeks objek di jaringan (0-based) |
| `iyres`, `mores` | tahun, bulan | `res_hyd%iyres`, `mores` | mulai beroperasi |
| `pvol`, `evol` | m3 | `res_ob%pvol`, `evol` | dari ha-m × 10.000 |
| `br1`, `br2` | – | `res_ob%br1`, `br2` | koefisien kurva volume–luas A = br1·V^br2 |
| `lag_up`, `lag_down` | – | `res_ob%lag_up`, `lag_down` | dari kolom `br1`, `br2` di `hydrology.res` |
| `prev_flo` | m3 | `res_ob%prev_flo` | pelepasan kemarin (setelah lag) |
| `k` | mm/jam | `res_hyd%k` | konduktivitas dasar (default 0,01) |
| `evrsv` | – | `res_hyd%evrsv` | koefisien evaporasi (default 0,7) |
| `alts`, `conds`, `acts` | – | `dtbl_res(irel)` | tabel keputusan terkompilasi |
| `prm` | – | `res_prm` | parameter sedimen/nutrien (`WbPrm`, 13.5) |
| `water` | `Hyd` | `res(jres)` | air dan muatannya dalam reservoir (m3, t, kg) |
| `wat` | `ChWat` | `res_wat_d(jres)` | luas (ha), presipitasi, evaporasi, rembesan (m3) hari ini |
| `in_d`, `out_d` | `Hyd` | `res_in_d`, `res_out_d` | inflow dan outflow hari ini untuk tabel keluaran |
| `nstep` | – | `time%step` | langkah waktu per hari (minimal 1) |
| `s_ini` | m3 | `res_ob%S_ini` | tampungan awal tahun operasi (Hanasaki) |

#### 13.1.1  `row`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `fn row<'a>(t: Option<&'a Table>, name: &str) -> Option<(&'a Table, usize)>`
- **Padanan SWAT+:** pencarian nama `do ... if (x(i)%name == nama) ... exit` yang tersebar di `res_read.f90`.
- **Tujuan:** mencari baris tabel teks berdasarkan kolom `name`.
- **Masukan dan keluaran:** tabel opsional (mis. `hydrology.res`, `initial.res`, `om_water.ini`) dan nama; mengembalikan tabel dan indeks baris pertama yang cocok, atau `None` bila tabel tidak ada atau nama tidak ditemukan.
- **Cara kerja:**
  1. Keluar dengan `None` bila tabel tidak ada.
  2. Menelusuri baris dari awal dan mengembalikan yang pertama dengan `fortran_eq(name, nama)`.
- **Kebiasaan SWAT+ / catatan port:** `fortran_eq` meniru perbandingan string Fortran (spasi di ujung diabaikan), sehingga nama yang berbeda hanya karena spasi penutup dianggap sama, seperti di Fortran. Baris pertama yang cocok menang, sesuai `exit` pada loop Fortran.

#### 13.1.2  `Reservoir::operational`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `fn operational(&self, yrc: i32, mo: i32) -> bool`
- **Padanan SWAT+:** syarat `if (time%yrc > res_hyd%iyres .or. (time%mo >= res_hyd%mores .and. time%yrc == res_hyd%iyres))` di `res_control.f90` dan `res_initial.f90`.
- **Tujuan:** menentukan apakah reservoir sudah beroperasi pada tahun kalender `yrc` dan bulan `mo`.
- **Masukan dan keluaran:** tahun kalender, bulan (1–12); `true` bila beroperasi.
- **Cara kerja:**
  1. Benar bila tahun sekarang sudah lewat tahun mulai `iyres`.
  2. Atau, pada tahun `iyres` itu sendiri, bila bulan sekarang sudah mencapai `mores`.
- **Kebiasaan SWAT+ / catatan port:** sebelum beroperasi, reservoir meneruskan inflow-nya tanpa perubahan (lihat 13.4). Saat inisialisasi syarat ini dievaluasi untuk bulan 1 tahun mulai simulasi; reservoir yang baru beroperasi kemudian tidak pernah mendapat air awal dari `initial.res` dan luas permukaannya mulai dari nol, persis seperti `res_initial`.

### 13.2  Inisialisasi reservoir

Inisialisasi menggabungkan tiga subrutin Fortran: `res_read` (mencocokkan nama di `reservoir.res` dengan baris `hydrology.res`, tabel pelepasan, `sediment.res`, `nutrients.res`, `initial.res`), `res_initial` (konversi satuan, kurva volume–luas, air awal) dan `res_convert_mass` (konsentrasi menjadi massa). Di port semuanya dilakukan dalam satu konstruktor yang dipanggil sekali per reservoir dari `route.rs`.

#### 13.2.1  `Reservoir::new`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `pub fn new(project: &Project, ires: usize, ob: usize, props: i32, yrc: i32) -> Result<Reservoir, String>`
- **Padanan SWAT+:** `res_read.f90` (bagian pencocokan nama), `res_initial.f90`, `res_convert_mass` (`hydrograph_module.f90`), serta pembacaan tabel keputusan di `dtbl_res_read` (string yang di Fortran ditafsirkan tiap hari di `conditions.f90` dan `res_hydro.f90`).
- **Tujuan:** membangun data dan keadaan awal reservoir ke-`ires` (0-based) yang berada di objek `ob`, dengan `reservoir.con` menunjuk baris `props` di `reservoir.res`, simulasi mulai tahun `yrc`.
- **Masukan dan keluaran:** `Project` (tabel `reservoir.res`, `hydrology.res`, `res_rel.dtl`, `initial.res`, `om_water.ini`, `sediment.res`, `nutrients.res`, `time.sim`); mengembalikan `Reservoir` atau pesan galat untuk fitur yang tidak didukung atau rujukan yang hilang.
- **Cara kerja:**
  1. **Dua baris `reservoir.res`.** Baris `own` dicari berdasarkan `id == ires + 1` (nomor reservoir itu sendiri) dan baris `pr` berdasarkan `id == props`. Dari `own` diambil nama hidrologi (`hyd`), sedimen (`sed`) dan nutrien (`nut`); dari `pr` diambil tabel pelepasan (`rel`) dan air awal (`init`).
  2. **Hidrologi.** Baris `hydrology.res` bernama `hyd` dicari dengan `row`; bila tidak ada, galat. Volume dikonversi dari ha-m ke m3:

```latex
V_e = 10000 \cdot evol_{ha\text{-}m}, \qquad V_p = 10000 \cdot pvol_{ha\text{-}m}
```

Luas `esa` dan `psa` (ha) dibaca apa adanya.

3. **Kurva volume–luas** A = br1 · V^br2 (A dalam ha, V dalam m3). Dengan selisih volume dalam ha-m `resdif = evol - pvol`:
   - Bila `esa - psa > 0` dan `resdif > 0`: hitung `lnvol = (log10 V_e - log10 V_p)` lalu potong ke bilangan bulat (`as i32`, padanan `integer :: lnvol` di Fortran). Bila `lnvol > 1e-4`, br2 = (log10 esa − log10 psa) / lnvol; selain itu br2 = (log10 esa − log10 psa) / 0,001. Bila br2 > 0,9, br2 dijepit ke 0,9 dan br1 = (psa/V\_p)^0,9; selain itu br1 = (esa/V\_e)^br2.
   - Selain itu: br2 = 0,9 dan br1 = (psa/V\_p)^0,9 bila V\_p > 1e−6, atau 0,1 bila tidak.

```latex
b_2 = \frac{\log_{10} A_e - \log_{10} A_p}{\lfloor \log_{10} V_e - \log_{10} V_p \rfloor}, \qquad b_1 = \left(\frac{A_e}{V_e}\right)^{b_2}
```

4. **Tabel pelepasan.** Nama `rel` yang diawali `ctbl_` ditolak (tabel kondisi Osvaldo, `res_rel_conds`). Tabel `res_rel.dtl` dengan nama itu dicari; bila tidak ada, galat.
5. **Kondisi.** Untuk setiap kondisi dan baris alternatifnya: kondisi pada objek lain (`ob_num != 0`) ditolak. Variabel `vol` dipetakan ke `Cond::Vol` dengan `lim_var` ∈ {`pvol`, `evol`, `e-pv`} dan operator ∈ {`*`, `+`, `-`, `/`}; untuk `e-pv` operator apa pun diterima karena tidak dipakai (Fortran juga mengabaikannya). Variabel `month` menjadi `Cond::Month`. Variabel lain ditolak.
6. **Aksi.** Untuk setiap aksi dan vektor hasilnya (`outcomes`), `option` dipetakan ke varian `Release` (tabel 13.1). Hal khusus:
   - `days` dan `dyrt` membaca basis dari `file_pointer` (`null`, `pvol`, `evol`; lainnya galat).
   - `dyrt1`: `file_pointer` `con1`, `con2`, `con3` menunjuk kondisi ke-5, ke-4, ke-3 (pemetaan terbalik yang dikodekan keras di Fortran dengan komentar "NAM setup"). Operator kondisi itu (`=`, `*`, `+`, `-`, `/`) dan `lim_const`-nya disalin ke varian; kondisi yang tidak ada atau operator lain ditolak.
   - `inflo_targ`: `const` aksi dibulatkan ke bilangan bulat `ic` dan `lim_const` kondisi ke-`ic` disalin; `ic < 1` atau di luar jumlah kondisi ditolak.
   - `irr-h06` ditolak (NaN di SWAT+); opsi lain (termasuk `weir`, `meas`) ditolak.
7. **Parameter sedimen/nutrien:** `WbPrm::new(project, sed, nut)` dengan nama dari baris `own` (13.5).
8. **Nilai awal keadaan:** `nstep = max(time.step, 1)`; `s_ini = max(V_e, V_p)`; `iyres`, `mores` dari `hydrology.res`; `lag_up` dan `lag_down` dari kolom `br1` dan `br2` file `hydrology.res`; `prev_flo = 0`; `k` (default 0,01 mm/jam) dan `evrsv` (default 0,7); hidrograf air, inflow, outflow dan `ChWat` nol.
9. **Air awal** (hanya bila `operational(yrc, 1)`): baris `initial.res` bernama `init` (dari baris `pr`) memberi nama `org_min`; baris `om_water.ini` dengan nama itu memberi fraksi volume `flo` dan konsentrasi. Volume awal flo = fraksi · V\_p; sedimen dan kelas butir (`sed`, `san`, `sil`, `cla`, `sag`, `lag`, `grv`) dalam ton = ppm · flo / 10^6; nutrien dan kualitas air (`orgn`, `sedp`, `no3`, `solp`, `chla`, `nh3`, `no2`, `cbod`, `dox`) dalam kg = mg/L · flo / 1000; `temp` disalin apa adanya. Luas awal `wat.area_ha = br1 · flo^br2`.

```latex
Q_0 = f_{flo} V_p, \quad M_{t} = \frac{c_{ppm} Q_0}{10^6}, \quad M_{kg} = \frac{c_{mg/L} Q_0}{1000}, \quad A_0 = b_1 Q_0^{b_2}
```

- **Kebiasaan SWAT+ / catatan port:**
  - **Nomor sendiri vs. `props`.** `res_read` mengisi `res_hyd(ires)` dan `res_prm(ires)` (sedimen dan nutrien) memakai indeks baris berulang `ires`, sedangkan tabel pelepasan dan air awal diambil lewat `res_ob%props`. Port meniru ini dengan dua baris `own` dan `pr`; SWAT+ Editor menulis keduanya dalam urutan yang sama sehingga biasanya sama (`docs/reservoirs.md`).
  - **`lnvol` bilangan bulat.** Karena dipotong, untuk V\_e < 10·V\_p hasilnya 0, cabang `/0,001` dipakai, br2 menjadi sangat besar lalu dijepit ke 0,9, sehingga luas efektif A = (psa/V\_p)^0,9 · V^0,9. Perhatikan pula bahwa `resdif` dihitung dalam ha-m tetapi `lnvol` dari volume m3 (keduanya hanya dipakai untuk tanda dan rasio, jadi konsisten dengan Fortran).
  - **`br1`/`br2` di `hydrology.res` adalah lag pelepasan**, bukan koefisien kurva; koefisien kurva selalu dihitung ulang (`docs/reservoirs.md`).
  - Logaritma memakai `f32::log10` dan pangkat memakai `rpow` agar sama dengan `Log10` dan `**` gfortran presisi tunggal.
  - Kebutuhan memori Hanasaki di `res_initial` (`I_mon_past` 60 bulan, `daily_inflow_array`, `c_ratio = 0,51`, dsb.) tidak dibawa ke port karena, seperti dijelaskan di 13.3, nilainya tidak pernah berubah dari nol pada kode acuan; hanya `S_ini` yang disimpan.
  - Pestisida, patogen, garam, dan konstituen (`res_pest`, `res_salt`, `res_cs`) tidak dibangun; port menjalankan reservoir tanpa konstituen.

### 13.3  Aturan pelepasan `res_rel.dtl`

Setiap hari volume pelepasan reservoir ditentukan oleh tabel keputusan: kondisi dievaluasi untuk menandai alternatif yang berlaku (`act_hit`), lalu setiap aksi yang hasilnya `y` pada salah satu alternatif yang berlaku dijumlahkan ke pelepasan. Di Fortran ini adalah pasangan `conditions(jres, irel)` lalu `res_hydro(jres, irel, pvol_m3, evol_m3)`; di port keduanya digabung dalam satu method privat.

#### 13.3.1  `Reservoir::release`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `fn release(&mut self, mo: i32, day: i32, day_mo: i32, hin: &Hyd) -> f32`
- **Padanan SWAT+:** `conditions.f90` (cabang `vol` dan `month`, dengan `cond_real`/`cond_integer`) dan `res_hydro.f90`.
- **Tujuan:** menghitung volume pelepasan hari ini (m3) sebelum lag, sedimen, dan neraca air.
- **Masukan dan keluaran:** bulan `mo`, hari Julian `day` (`time%day`), hari dalam bulan `day_mo`, hidrograf masuk hari ini `hin` (m3 dan muatan); state yang dibaca: `water.flo` (volume tampungan setelah inflow hari ini ditambahkan), `pvol`, `evol`, `nstep`, `s_ini`. Keluaran: pelepasan Q (m3/hari). Efek samping: `s_ini` dapat diperbarui oleh `nonirr-h06`.
- **Cara kerja:**
  1. **Kondisi.** Vektor `hit` berisi `true` untuk setiap alternatif. Volume sekarang `cur = water.flo`. Untuk setiap kondisi:
     - `Vol` dengan `EmPv`: target = pvol + (evol − pvol)·lim.
     - `Vol` dengan `Pvol`/`Evol` (V = pvol atau evol): `*` → V·lim; `+` → V + 10.000·lim; `-` → V − 10.000·lim; `/` → V/lim.
     - `Month`: target = lim, nilai sekarang = `mo`.
     - `cond_test` (dari `hru/mgt.rs`) lalu mematikan setiap alternatif yang operatornya (`<`, `>`, `<=`, `>=`, `=`, `/=`) gagal; operator `-` dan alternatif yang sudah mati dilewati.
  2. **Loop aksi.** Aksi diulang `nstep` kali secara berurutan (`acts.iter().cycle().take(len · nstep)`, setara loop luar `tstep` dan loop dalam `iac` di Fortran). Aksi diambil bila tabel tidak punya alternatif (`alts == 0`) atau ada alternatif k dengan `hit[k]` dan hasil aksi `y`.
  3. **Penjumlahan per opsi** (Q bertambah; `ns = nstep`; `cur` tetap volume awal hari ini, tidak dikurangi selama loop):
     - `rate`: Q += r·86400 (m3/s ke m3/hari).
     - `rate_pct`: Q += p·pvol/100.
     - `inflo_rate`: Q += max(inflow + plus·86400, min·86400).
     - `inflo_frac`: Q += inflow·f.
     - `ab_emer`: bila cur > evol, Q += cur − evol.
     - `days`: b\_lo = 0 untuk `null`, atau basis·frac; Q += (cur − b\_lo)/days/ns; lalu Q = max(Q, 0).
     - `dyrt`: b\_lo = max(basis, 0); Q += (cur − b\_lo)/days + pct·pvol/100/ns; Q = max(Q, 0).
     - `dyrt1`: b\_lo dari operator kondisi rujukan: `=` → pvol + (evol − pvol)·lim; `*` → (evol − pvol)·lim; `+` → (evol − pvol) + lim; `-` → (evol − pvol) − lim; `/` → (evol − pvol)/lim; dijepit ≥ 0; lalu rumus `dyrt`.
     - `inflo_targ`: b\_lo = pvol·lim; Q += inflow + (cur − b\_lo); Q = max(Q, 0).
     - `natlake`: So = so\_frac·pvol, Smax = pvol, S = cur − So; Q += (S/(Smax − So))^1,5 · S · kr.
     - `nonirr-h06`: Smax = max(pvol, evol); rata-rata inflow I\_mean = 0; bila `day_mo == 1` dan cur > pvol maka `s_ini = cur`; er = s\_ini/(alpha·Smax); Q += er·I\_mean.
     - `hydrop`: S\_min = 0,2·pvol, S\_lim = 0,5·pvol; F\_lin = clamp((cur − S\_min)/(S\_lim − S\_min), 0, 1); F\_sin = max(0, 1 + a·(2π·day + b)/365); Q += F\_lin·F\_sin·I\_mean dengan I\_mean = 0.
  4. Mengembalikan Q.

Persamaan utama:

```latex
Q_{days} = \frac{V - b_{lo}}{n_{days}\, n_{step}}, \qquad Q_{dyrt} = \frac{V - b_{lo}}{n_{days}} + \frac{p\, V_p}{100\, n_{step}}
```

```latex
Q_{natlake} = k_r \left(\frac{S}{S_{max} - S_o}\right)^{1.5} S, \qquad S = V - S_o,\; S_o = f_o V_p,\; S_{max} = V_p
```

```latex
Q_{H06} = \frac{S_{ini}}{\alpha \max(V_p, V_e)} \bar I, \qquad Q_{hydrop} = F_{lin} F_{sin} \bar I, \quad F_{sin} = \max\left(0, 1 + a \frac{2\pi d + b}{365}\right)
```

- **Kebiasaan SWAT+ / catatan port:**
  - **Satuan kondisi `vol`:** operator `+` dan `-` bekerja dalam ha-m (dikali 10.000 m3), sedangkan `*` dan `/` adalah fraksi (`docs/reservoirs.md`).
  - **Setiap aksi diambil sekali per langkah waktu.** Dengan `nstep > 1`, opsi yang tidak membagi dengan `nstep` (`rate`, `rate_pct`, `inflo_rate`, `inflo_frac`, `ab_emer`, `inflo_targ`, suku pertama `dyrt`/`dyrt1`, `natlake`) terhitung `nstep` kali; misalnya `inflo_targ` melepas inflow hari itu `step` kali (`docs/engine1.md`). Hanya `days` dan suku persentase `dyrt`/`dyrt1` yang dibagi `nstep`.
  - **Penjepitan `max(0, Q)` bersifat kumulatif:** pada `days`, `dyrt`, `dyrt1`, `inflo_targ` yang dijepit adalah jumlah Q sejauh ini, bukan kontribusi aksi itu saja, sesuai `ht2%flo = max(0., ht2%flo)` di Fortran. Urutan aksi di tabel karenanya memengaruhi hasil.
  - **Danau alami di bawah volume mati melepas NaN:** S negatif dipangkatkan 1,5 menghasilkan NaN, yang lalu merambat ke hilir (`docs/engine1.md`). Port mempertahankannya (pangkat lewat `rpow`).
  - **`nonirr-h06` dan `hydrop` melepas nol.** Kedua skema memakai rata-rata inflow bulanan lima tahun terakhir (`I_mon_past`), yang di `res_control` hanya diisi bila variabel lokal `dom` dan `end_of_mo` bernilai 1 dan dari `daily_inflow`; ketiganya tidak pernah di-set (tetap 0), sehingga rata-ratanya selalu 0. Port karenanya memakai `i_mean = 0` langsung. Pembaruan `S_ini` tetap ditiru: syarat Fortran "I\_mon < I\_mean" (0 < 0) selalu salah, jadi hanya syarat cur > pvol pada hari pertama bulan yang berlaku. Di Fortran, `c_ratio = Smax/(I_mean·365,25)` menjadi tak hingga sehingga cabang `c_ratio >= 0,5` (penambahan er·I\_mean) yang dipakai, sama dengan port.
  - **Faktor musiman `hydrop` bukan sinus:** Fortran menulis `1 + a·((2π·day + b)/365)` tanpa fungsi `sin`; port mengikutinya dan memberi komentar "Not a sine". Karena dikalikan I\_mean = 0, faktor ini tidak berpengaruh pada hasil, tetapi akan berpengaruh bila kelak I\_mean diperbaiki.
  - **`dyrt1` dan `con1..con3`:** pemetaan ke kondisi ke-5, ke-4, ke-3 dikodekan keras di Fortran; di Fortran `file_pointer` lain membiarkan `ic` dari nilai sebelumnya, port menolaknya saat inisialisasi.
  - Nilai `π` di port adalah `std::f32::consts::PI`; di Fortran konstanta `real` yang diinisialisasi dari literal panjang, yang dibulatkan ke nilai `f32` yang sama.
  - Varian `days`/`dyrt` dengan basis `null`: pada `days` b\_lo = 0 tanpa dikali frac; pada `dyrt` b\_lo = 0. Fraksi `const2` pada `days` hanya berlaku untuk basis `pvol`/`evol`.

### 13.4  Neraca air harian reservoir

Neraca harian menggabungkan inflow, pelepasan menurut tabel, peredaman dengan lag, pengendapan sedimen, presipitasi langsung, evaporasi, rembesan dasar, pembaruan luas permukaan, dan pengendapan nutrien. Pemanggil di `route.rs` (cabang `ObjectKind::Reservoir`) hanya memanggil `control` bila objek punya inflow (`ins` tidak kosong, padanan `if (ob(icmd)%rcv_tot > 0)` di `command.f90`), dan menyerahkan presipitasi, PET dan suhu rata-rata stasiun cuaca objek (nol bila stasiun tidak ada).

#### 13.4.1  `Reservoir::control`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `pub fn control(&mut self, hin: Hyd, yrc: i32, mo: i32, day: i32, day_mo: i32, precip: f32, pet: f32, tave: f32, print: bool) -> Hyd`
- **Padanan SWAT+:** `res_control.f90` (cabang `rel_tbl == "d"`, tanpa `gwflow`, tanpa konstituen), memanggil `conditions`, `res_hydro`, `res_sediment`, `res_nutrient`.
- **Tujuan:** menjalankan satu hari reservoir dan mengembalikan hidrograf keluar.
- **Masukan dan keluaran:** `hin` inflow hari ini (m3, t, kg); tahun `yrc`, bulan `mo`, hari Julian `day`, hari dalam bulan `day_mo`; presipitasi `precip` (mm), PET `pet` (mm), suhu rata-rata `tave` (°C); `print` bila tahun simulasi sudah melewati `nyskip`. Keluaran: hidrograf `ht2` (m3 air, t sedimen, kg nutrien). State yang berubah: `water`, `wat`, `prev_flo`, `s_ini`, `in_d`, `out_d`.
- **Cara kerja:**
  1. Bila belum `operational(yrc, mo)`, kembalikan `hin` apa adanya (inflow diteruskan).
  2. `ht2` = hidrograf nol. Tambahkan inflow ke tampungan: `water = water + hin` (`Hyd::plus`).
  3. Pelepasan mentah `ht2.flo = release(mo, day, day_mo, &hin)` (13.3).
  4. **Lag pelepasan:** α\_up = exp(−lag\_up), α\_down = exp(−lag\_down). Bila pelepasan kemarin < pelepasan hari ini, Q = Q·α\_up + Q\_kemarin·(1 − α\_up); selain itu dengan α\_down. Simpan `prev_flo = Q`.
  5. `res_sediment(&mut water, &mut ht2, &prm)` (13.5): sedimen mengendap dan outflow mengambil konsentrasi setelah pengendapan.
  6. **Fluks vertikal** dari luas permukaan A (ha) yang tersimpan dari akhir hari sebelumnya: evaporasi E = 10·evrsv·PET·A, presipitasi P = 10·precip·A, rembesan S = 240·k·A (semua m3).
  7. **Neraca berurutan dengan pemotongan:** V += P; V −= Q, bila V < 0 maka Q += V dan V = 0; V −= E, bila V < 0 maka E += V dan V = 0; V −= S, bila V < 0 maka S += V dan V = 0.
  8. **Luas baru:** A = br1·V^br2 bila V > 0, selain itu 0.
  9. `res_nutrient(&mut water, &mut ht2, &prm, mo, tave)` (13.5).
  10. Bila `print`, simpan `in_d = hin` dan `out_d = ht2` untuk tabel `reservoir_*`.
  11. Kembalikan `ht2`.

```latex
Q_t = \begin{cases} \alpha_{up} Q^* + (1-\alpha_{up}) Q_{t-1} & Q_{t-1} < Q^* \\ \alpha_{down} Q^* + (1-\alpha_{down}) Q_{t-1} & \text{lainnya} \end{cases}, \qquad \alpha = e^{-lag}
```

```latex
E = 10\, \eta_{ev}\, PET\, A, \qquad P = 10\, R\, A, \qquad S = 240\, K\, A, \qquad A = b_1 V^{b_2}
```

Faktor 10 berasal dari 1 mm di atas 1 ha = 10 m3, dan 240 = 24 jam × 10 m3/(ha·mm), sehingga K dalam mm/jam menjadi m3/hari.

- **Kebiasaan SWAT+ / catatan port:**
  - **Urutan operasi** identik dengan `res_control`: inflow ditambahkan dulu, pelepasan dihitung dari tampungan termasuk inflow, sedimen diendapkan sebelum neraca air (jadi konsentrasi sedimen dihitung dengan volume sebelum presipitasi, pelepasan, evaporasi, dan rembesan), nutrien diendapkan sesudahnya dengan volume akhir. Urutan pemotongan Q → E → S berarti pelepasan didahulukan atas evaporasi dan rembesan bila air tidak cukup.
  - **Luas untuk fluks hari ini adalah luas kemarin** (luas diperbarui setelah neraca), seperti di Fortran.
  - **Suhu outflow 0 °C:** `ht2` dimulai dari hidrograf nol (`resz`) dan tidak ada yang mengisi `temp`, lalu sungai di hilir mencampurnya (`docs/reservoirs.md`).
  - **`cbod` dan `dox`** tidak dipindahkan ke outflow oleh `res_nutrient`, sehingga outflow selalu nol untuk keduanya dan keduanya menumpuk di tampungan (perilaku kode acuan, bukan koreksi port).
  - **Reservoir tanpa inflow tidak dijalankan** sama sekali (termasuk evaporasi dan rembesan), dan `basin_reservoir_output` me-nol-kan `res_in_d`/`res_out_d` setelah menjumlahkan sehingga reservoir yang tidak dijalankan mencetak nol (`docs/reservoirs.md`).
  - Hal yang dilakukan `res_control` tetapi tidak di method ini: koreksi *lapse rate* (port menolak reservoir dengan `lapse`), `wallo_control` (alokasi air), pelacakan `daily_inflow_array`/`I_mon_past` (selalu nol, lihat 13.3), cabang `res_rel_conds` (`ctbl_`, ditolak), `gwflow_reservoir` (port selalu memakai rembesan 240·k·A), pestisida/garam/konstituen, akumulasi `hin_tot`/`hout_tot`, pembagian `ht2` ke langkah subharian `ob%ts`, dan `bsn_sedbud%res_dep_t`. Apakah bagian yang tersisa itu ditangani di tempat lain di port tidak dibahas di bab ini.
  - `exp` memakai `f32::exp`, sama dengan `Exp` presisi tunggal Fortran.

### 13.5  Sedimen dan nutrien badan air (`waterbody.rs`)

File `waterbody.rs` berisi apa yang dipakai bersama oleh reservoir (`res.rs`) dan lahan basah HRU (`hru/wetland.rs`, yang memanggil `res_sediment` dan `res_nutrient` dengan air lahan basah dan outflow-nya). Di Fortran keduanya bekerja lewat pointer global `wbody`, `wbody_wb`, `wbody_prm`, dan hidrograf modul `ht2`; di port semuanya parameter eksplisit.

**Struct `WbPrm`** (publik, padanan `water_body_data_parameters` di `reservoir_data_module.f90`; laju per hari):

| Field | Satuan | Sumber | Keterangan |
| --- | --- | --- | --- |
| `nsed` | mg/L | `sediment.res` | konsentrasi sedimen normal (keseimbangan) |
| `sed_stlr_co` | – | dihitung | exp(−0,184·d50), d50 dalam µm; 0 bila baris sediment.res tidak ditemukan (begitu pula soln\_stl\_fr dan solp\_stl\_fr, yang bernilai 0,2 bila baris ditemukan) |
| `ires1`, `ires2` | bulan | `nutrients.res` | awal dan akhir musim pengendapan pertengahan tahun |
| `nsetlr1`, `nsetlr2` | per hari | `nutrients.res` / 365 | laju kehilangan N organik (musim, sisa tahun) |
| `psetlr1`, `psetlr2` | per hari | `nutrients.res` / 365 | laju kehilangan P sedimen |
| `nsolr`, `psolr` | per hari | `nutrients.res` / 365 | laju kehilangan N terlarut (no3, nh3, no2) dan P terlarut |
| `theta_n`, `theta_p` | – | `nutrients.res` | koefisien koreksi suhu (default 1) |
| `conc_nmin`, `conc_pmin` | mg/L | `nutrients.res` | konsentrasi minimum untuk pengendapan (default 0,1 dan 0,01) |

#### 13.5.1  `find`

- **Lokasi:** `crates/swat-engine/src/waterbody.rs`, `fn find<'a>(t: Option<&'a Table>, name: &str) -> Option<(&'a Table, usize)>`
- **Padanan SWAT+:** loop pencocokan nama di `res_read.f90` (dan `wet_read`).
- **Tujuan:** mencari baris pertama tabel dengan kolom `name` yang sama (`fortran_eq`). Identik dengan `row` di `res.rs` (13.1.1); keduanya duplikat kecil per modul.
- **Masukan dan keluaran:** tabel opsional, nama; tabel dan indeks baris, atau `None`.

#### 13.5.2  `WbPrm::new`

- **Lokasi:** `crates/swat-engine/src/waterbody.rs`, `pub fn new(project: &Project, sed: &str, nut: &str) -> WbPrm`
- **Padanan SWAT+:** `res_read_sed.f90`, `res_read_nut.f90` (konversi per tahun ke per hari), dan blok pencocokan `res_sed`/`res_nut` di `res_read.f90` (yang juga menetapkan `sed_stlr_co`, `soln_stl_fr`, `solp_stl_fr`).
- **Tujuan:** menyusun parameter sedimen dan nutrien sebuah badan air dari baris bernama `sed` di `sediment.res` dan `nut` di `nutrients.res`.
- **Masukan dan keluaran:** proyek, nama baris sedimen dan nutrien; `WbPrm`.
- **Cara kerja:**
  1. Baris sedimen: `nsed` dan `d50` (0 bila baris tidak ditemukan). Bila baris ditemukan, `soln_stl_fr` = `solp_stl_fr` = 0,2; bila tidak, keduanya dan `sed_stlr_co` bernilai 0, sama dengan `res_read` dan `wet_initial`.
  2. Baris nutrien (opsional). `ires1`, `ires2` dibaca sebagai bilangan bulat (default 0). Laju `nsetlr1`, `nsetlr2`, `psetlr1`, `psetlr2`, `nsolr`, `psolr` dibagi 365 bila barisnya ada; bila baris tidak ada dipakai default tipe Fortran tanpa pembagian (0 untuk laju pengendapan, 1 untuk `nsolr`/`psolr`). `theta_n`, `theta_p` (default 1), `conc_nmin` (0,1), `conc_pmin` (0,01) tidak dibagi.
  3. Koefisien pengendapan sedimen dari ukuran butir median (hanya bila baris sedimen ditemukan; bila tidak, 0):

```latex
c_{stl} = e^{-0.184\, d_{50}}
```

- **Kebiasaan SWAT+ / catatan port:**
  - Pembagian 365 terjadi saat membaca file (`res_read_nut`), sehingga hanya nilai dari file yang dibagi; default tipe (`nsolr = 1`) tidak dibagi. Port meniru dengan closure `rate` yang membagi hanya bila baris ada.
  - Fraksi pengendapan nutrien terlarut `soln_stl_fr` dan `solp_stl_fr` disimpan di struct sejak 1.0.1 dan dipakai oleh `res_nutrient` (sebelumnya konstanta 0,2 di fungsi itu). Seperti di Fortran, keduanya bernilai 0,2 hanya di dalam blok "baris sedimen ditemukan" (beserta `sed_stlr_co`); bila nama `sed` reservoir tidak ada di `sediment.res`, ketiganya 0, sama dengan `res_read` dan `wet_initial`. Sebelum 1.0.1 port memakai exp(0) = 1 dan 0,2 pada kasus itu; kini diperbaiki di 1.0.1, sesuai SWAT+, dan divalidasi oleh varian Bh\_nosed (Bh\_res dengan reservoir tanpa baris sedimen) dan Ps\_edge.
  - Komentar modul Fortran menyebut `nsed` "dibaca mg/L dan dikonversi ke kg/L", tetapi tidak ada konversi di `res_read_sed`; nilai dipakai sebagai mg/L (ppm) langsung, dan port melakukan hal yang sama.

#### 13.5.3  `theta`

- **Lokasi:** `crates/swat-engine/src/waterbody.rs`, `fn theta(r20: f32, thk: f32, tmp: f32) -> f32`
- **Padanan SWAT+:** fungsi `theta` di `theta.f90` (persamaan III-52 QUAL2E).
- **Tujuan:** mengoreksi laju reaksi pada 20 °C ke suhu `tmp`.
- **Masukan dan keluaran:** laju pada 20 °C (per hari), faktor koreksi (–), suhu (°C); laju terkoreksi (per hari).
- **Cara kerja:** satu baris, dengan pangkat riil lewat `rpow`:

```latex
r(T) = r_{20}\, \theta^{\,T - 20}
```

#### 13.5.4  `res_sediment`

- **Lokasi:** `crates/swat-engine/src/waterbody.rs`, `pub fn res_sediment(b: &mut Hyd, out: &mut Hyd, p: &WbPrm)`
- **Padanan SWAT+:** `res_sediment.f90`.
- **Tujuan:** mengendapkan sedimen tersuspensi di badan air menuju konsentrasi normalnya dan menetapkan muatan sedimen outflow.
- **Masukan dan keluaran:** `b` air badan air (m3, sedimen t), `out` outflow hari ini (volume sudah ditetapkan); `p.nsed` (mg/L), `p.sed_stlr_co`. Mengubah `b.sed`, kelas butir `b`, dan `out.sed` (t).
- **Cara kerja:**
  1. Bila volume < 1e−6 m3: seluruh `b` di-nol-kan dan `out.sed = 0`.
  2. Selain itu konsentrasi C = max(10^6·sed/V, 1e−6) (mg/L, karena t/m3 × 10^6 = g/m3).
  3. Bila C > nsed: C' = (C − nsed)·c\_stl + nsed, dijepit ≥ nsed; sedimen tampungan = C'·V/10^6; sedimen outflow = C'·Q/10^6; sedimen tampungan dikurangi sedimen outflow (≥ 0); kelas `sil`, `cla`, `san`, `sag`, `lag`, `grv` tampungan di-nol-kan.
  4. Bila C ≤ nsed: tidak ada yang berubah; `out.sed` tetap nilai sebelumnya (nol, karena `ht2` dimulai dari nol).

```latex
C' = (C - C_{n})\, e^{-0.184 d_{50}} + C_n, \qquad sed_{out} = \frac{C' Q}{10^6}, \qquad sed_{res} = \max\left(0, \frac{C' V}{10^6} - sed_{out}\right)
```

- **Kebiasaan SWAT+ / catatan port:**
  - **Sedimen hanya mengendap di atas konsentrasi normal**; pada atau di bawahnya outflow tidak membawa sedimen sama sekali, dan kelas butir tetap di air (`docs/reservoirs.md`). Akibatnya reservoir Bh\_res menahan sekitar 99 % sedimen yang masuk.
  - **Kelas butir dibuang, bukan diendapkan proporsional**: begitu ada pengendapan, semua kelas tampungan menjadi nol dan outflow juga tidak membawa kelas butir (hanya `sed`).
  - Volume V yang dipakai adalah volume sebelum presipitasi, pelepasan, evaporasi, dan rembesan dikurangkan (lihat urutan di 13.4), sehingga sedimen outflow dihitung atas Q tetapi dikurangkan dari tampungan yang belum dikurangi Q.
  - Fortran juga menghitung `sil_ppm` dan `cla_ppm` yang tidak dipakai; port menghilangkannya tanpa mengubah hasil.

#### 13.5.5  `res_nutrient`

- **Lokasi:** `crates/swat-engine/src/waterbody.rs`, `pub fn res_nutrient(b: &mut Hyd, o: &mut Hyd, p: &WbPrm, mo: i32, tave: f32)`
- **Padanan SWAT+:** `res_nutrient.f90`.
- **Tujuan:** mengendapkan N organik, P sedimen, serta N dan P terlarut di badan air, menghitung klorofil-a semu, lalu membagi nutrien tersisa antara tampungan dan outflow menurut proporsi volume.
- **Masukan dan keluaran:** `b` air badan air (volume akhir hari, m3; nutrien kg), `o` outflow (volume Q), bulan `mo`, suhu rata-rata stasiun `tave` (°C). Mengubah `no3`, `nh3`, `no2`, `orgn`, `sedp`, `solp`, `chla` pada `b` dan `o`.
- **Cara kerja:**
  1. Bila V < 1e−6: `b` di-nol-kan, keluar (outflow tetap tanpa nutrien).
  2. Pilih laju musim: bila `ires1 ≤ mo ≤ ires2` pakai `nsetlr1`, `psetlr1`, selain itu `nsetlr2`, `psetlr2`.
  3. Konsentrasi (mg/L = 1000·kg/m3): C\_N = orgn, C\_P = sedp, C\_solN = no3 + nh3 + no2, C\_solP = solp, masing-masing dibagi V.
  4. Koefisien kehilangan, masing-masing dijepit ke \[0, 1\]: nitrok dari (C\_N − conc\_nmin)·θ(nsetlr, θ\_n, T); phosk dari (C\_P − conc\_pmin)·θ(psetlr, θ\_p, T); nitrosolk dari (C\_solN − conc\_nmin)·θ(nsolr, θ\_n, T); phossolk dari (C\_solP − conc\_pmin)·θ(psolr, θ\_p, T).
  5. Pengurangan: solp ×= 1 − solp\_stl\_fr·phossolk; sedp ×= 1 − phosk; orgn ×= 1 − nitrok; no3, nh3, no2 masing-masing ×= 1 − soln\_stl\_fr·nitrosolk.
  6. Klorofil-a: chla = 0; tpco = 10^6·(solp + sedp)/(V + Q); bila tpco > 1e−4 maka chla = (V + Q)·10^−6.
  7. Semua tujuh komponen dijepit ≥ 0.
  8. Bagian outflow: X\_out = X·Q/(V + Q) untuk no3, orgn, sedp, solp, chla, nh3, no2; lalu X\_res = max(0, X − X\_out).

```latex
k = \min\left(1, \max\left(0, (C - C_{min})\, r_{20}\, \theta^{\,T-20}\right)\right), \qquad X \leftarrow X (1 - f\, k)
```

```latex
X_{out} = X \frac{Q}{V + Q}, \qquad X_{res} = \max(0, X - X_{out})
```

dengan f = 1 untuk orgn dan sedp, f = soln\_stl\_fr atau solp\_stl\_fr (0,2, atau 0 bila baris sediment.res tidak ditemukan) untuk nutrien terlarut.

- **Kebiasaan SWAT+ / catatan port:**
  - Suhu yang dipakai adalah suhu udara rata-rata stasiun (`wst%weat%tave`), bukan suhu air; port menerima `tave` dari pemanggil (reservoir: stasiun objek; lahan basah: stasiun HRU).
  - **Klorofil-a bukan model alga**: nilainya hanya (V + Q)·10^−6 kg bila ada P, lalu dibagi seperti nutrien lain. Ini perilaku kode acuan apa adanya.
  - Pembagian nutrien memakai V + Q, padahal V sudah volume setelah Q dikeluarkan (13.4); jadi proporsinya adalah Q terhadap volume sebelum pelepasan (tanpa evaporasi dan rembesan).
  - `cbod`, `dox`, dan kelas butir tidak disentuh; outflow `cbod`/`dox` tetap nol.
  - Penjepitan ditulis `min(1)` lalu `max(0)`, mengikuti urutan Fortran `amin1` lalu `max`. Perilaku keduanya terhadap NaN (mis. NaN dari `natlake` yang merambat) belum diverifikasi penulis; untuk nilai berhingga hasilnya identik.
  - Port tetap memakai f32 di semua pembagian, termasuk 1000·kg/V, dalam urutan yang sama.

### 13.6  Objek recall (sumber titik dan aliran terukur)

Objek *recall* (`recall.con`, `recall.rec`, dan file data yang dirujuknya) memasukkan beban yang diukur atau dimodelkan di luar SWAT+ ke jaringan: buangan instalasi pengolahan, sumber titik, atau aliran terukur di batas hulu. Data dibaca seluruhnya saat inisialisasi ke tabel per langkah dan per tahun simulasi; setiap hari perintah `recall` hanya mengambil satu entri.

**Struct `Recall`** (publik):

| Field | Keterangan |
| --- | --- |
| `ob` | indeks objek di jaringan |
| `typ` | 1 harian, 2 bulanan, 3 tahunan (0 subharian dan 4 `exco` tidak didukung) |
| `start_yr`, `end_yr` | tahun kalender pertama di file dan tahun terakhir yang dibaca |
| `steps` | langkah per tahun: 366, 12, atau 1 |
| `hd` | `Vec<Hyd>` berukuran steps × nbyr, diindeks (step − 1) + steps·(iyrs − 1); padanan `recall%hd(step, iyrs)` |
| `d` | beban hari ini (`rec_d`) untuk tabel `recall_*` dan `basin_psc_*` |

#### 13.6.1  `Recall::new`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `pub fn new(project: &Project, ob: usize, props: i32, yrc: i32, yrc_end: i32, nbyr: usize) -> Result<Recall, String>`
- **Padanan SWAT+:** `recall_read.f90` (bagian pembacaan `recall.rec` dan file datanya; pembacaan teks file sendiri ada di crate `swat-model`).
- **Tujuan:** menyusun tabel beban recall untuk objek `ob` yang menunjuk baris `props` di `recall.rec`, untuk simulasi dari `yrc` sampai `yrc_end` (`nbyr` tahun).
- **Masukan dan keluaran:** proyek (daftar `project.recalls`, masing-masing dengan `id`, `name`, `typ`, dan baris data `iyr`, `jday`, `mo`, `vals[0..18]`), indeks objek, nomor baris, tahun awal/akhir, jumlah tahun; `Recall` atau galat.
- **Cara kerja:**
  1. Cari rekaman `recall.rec` dengan `id == props`; galat bila tidak ada.
  2. Tentukan `steps` dari `typ`: 1 → 366, 2 → 12, 3 → 1; tipe 0 (subharian) dan lainnya ditolak. Alokasikan `hd` nol sebanyak steps·max(nbyr, 1).
  3. `start_yr` = tahun baris pertama file. Bila `start_yr ≤ yrc`: lompat ke baris pertama dengan tahun = `yrc` (galat bila tidak ada), `iyrs = 1`. Bila file mulai setelah simulasi: mulai dari baris pertama dengan `iyrs = start_yr − yrc + 1`.
  4. Untuk setiap baris berikutnya: simpan tahun sebelumnya, baca tahun baris; berhenti bila tahun > `yrc_end`; bila tahun berganti, `iyrs += 1`. Langkah = `jday` (harian), `mo` (bulanan), atau 1 (tahunan).
  5. Aliran negatif (diversi) ditolak. Nilai `vals` dipetakan berurutan ke `flo`, `sed`, `orgn`, `sedp`, `no3`, `solp`, `chla`, `nh3`, `no2`, `cbod`, `dox`, `san`, `sil`, `cla`, `sag`, `lag`, `grv`, `temp`.
  6. Simpan ke `hd[(step − 1) + steps·(iyrs − 1)]`; langkah atau tahun di luar rentang menjadi galat.
  7. `end_yr` = tahun baris terakhir yang dibaca (termasuk baris yang menghentikan loop karena melewati `yrc_end`).
- **Kebiasaan SWAT+ / catatan port:**
  - **`end_yr` dapat melebihi `yrc_end`**, karena Fortran membaca tahun baris pemutus sebelum keluar; port meniru dengan mengisi `iyr` sebelum `break`.
  - Aliran di file berupa m3 per langkah (per hari untuk tipe harian), walaupun tabel keluaran memberi judul `m^3/s` (`docs/reservoirs.md`).
  - Fortran mengalokasikan `hd` dengan `time%nbyr` dan menulis di luar batas bila file tidak sesuai; port memeriksa rentang dan mengembalikan galat.
  - `recall.rec` dibaca terlepas dari ada tidaknya objek recall, dan pembacaan berhenti pada file pertama yang hilang atau berakhir sebelum header (kasus `RECYR.` di Osu\_1hru) (`docs/reservoirs.md`; ditangani di `swat-model`).
  - Diversi (`recall_nut` untuk aliran negatif), tipe subharian, dan tipe 4 (`exco`) tidak didukung.

#### 13.6.2  `Recall::day`

- **Lokasi:** `crates/swat-engine/src/res.rs`, `pub fn day(&mut self, day: i32, mo: i32, yrc: i32, yrs: i32) -> Hyd`
- **Padanan SWAT+:** cabang `case ("recall")` di `command.f90`.
- **Tujuan:** memberikan beban recall hari ini sebagai hidrograf keluar objek.
- **Masukan dan keluaran:** hari Julian `day`, bulan `mo`, tahun kalender `yrc`, tahun simulasi `yrs` (1-based); hidrograf beban hari itu, juga disimpan di `self.d`.
- **Cara kerja:**
  1. Fungsi bantu `at(step)` mengambil `hd[(step − 1) + steps·(yrs − 1)]` (step dan yrs minimal 1), atau hidrograf nol bila indeks di luar vektor.
  2. `within` = `start_yr ≤ yrc ≤ end_yr`.
  3. Harian: `at(day)` bila `within`, selain itu nol. Bulanan: `at(mo)` bila `within`, selain itu nol. Tahunan: selalu `at(1)`.
  4. Simpan ke `d` dan kembalikan.
- **Kebiasaan SWAT+ / catatan port:**
  - **Recall tahunan menaruh beban setahun penuh di setiap hari:** Fortran mengambil `recall%hd(1, yrs)` setiap hari, dan uji tahunnya memakai `.or.` (`yrc >= start_yr .or. yrc <= end_yr`) sehingga selalu benar. Port mempertahankan ini (komentar "`.or.`: always"). Pengguna yang ingin beban tahunan terbagi harian harus membaginya sendiri di file.
  - Recall harian dan bulanan memberi nol di luar tahun file.
  - Indeks di luar vektor menghasilkan nol di port (`unwrap_or_default`), padahal di Fortran merupakan akses di luar batas array.

### 13.7  Ringkasan fitur, batasan, dan validasi

| Aspek | Status di port | Rujukan |
| --- | --- | --- |
| Opsi pelepasan `rate`, `rate_pct`, `inflo_rate`, `inflo_frac`, `ab_emer`, `days`, `dyrt` | didukung, bit-for-bit | `docs/reservoirs.md`, Bh\_res |
| `dyrt1`, `inflo_targ`, `natlake`, `nonirr-h06`, `hydrop` | didukung, bit-for-bit (dua terakhir melepas 0) | `docs/engine1.md`, Bh\_rel |
| `irr-h06` | ditolak (SWAT+ menghasilkan NaN) | `docs/engine1.md` |
| `weir`, `meas`, tabel kondisi `ctbl_` | ditolak | `docs/engine1.md`, `docs/reservoirs.md` |
| Kondisi selain `vol`/`month`, kondisi pada objek lain | ditolak | `Reservoir::new` |
| Reservoir dengan *lapse rate* | ditolak | `route.rs` |
| Recall subharian, diversi (aliran negatif), `exco` | ditolak | `docs/reservoirs.md` |
| Pestisida, garam, konstituen reservoir | tidak dimodelkan | – |

**Validasi.** Tidak ada proyek acuan yang memiliki reservoir atau recall, sehingga `scripts/make-reservoir-variants.py` membangun varian Batanghari:

- **Bh\_res** (2015–2017): satu reservoir hipotetis `res001` di hulu saluran cha037 dengan tabel pelepasan `flood_season` milik Batanghari, plus recall harian `mill` (efluen pabrik kelapa sawit) dan recall tahunan `town` (air limbah kota). Semua tabel cocok dengan SWAT+, termasuk tabel reservoir, recall, jumlah basin, dan 284 saluran; tampungan berkisar 47–53 juta m3 dan sekitar 99 % sedimen yang masuk tertahan. Varian Bh\_nosed (1.0.1) adalah Bh\_res dengan reservoir yang menunjuk baris sediment.res yang tidak ada, sehingga faktor pengendapannya 0 seperti di SWAT+; semua tabelnya juga cocok.
- **Bh\_rel**: Bh\_res dengan tabel `lake_multi`: di bawah pelimpah utama sebagai danau alami plus skema Hanasaki dan PLTA serta pelepasan minimum 2 m3/s; di antara pelimpah `dyrt1` 30 hari menuju tampungan banjir kondisi ketiga plus 1 % pvol per hari; di atas pelimpah darurat `inflo_targ`. Tampungan berayun antara kosong dan 48,5 juta m3; semua tabel, termasuk tabel harian reservoir, cocok dengan SWAT+.

Tabel keluaran (`reservoir_*`, `basin_res_*`, `recall_*`, `basin_psc_*`, harian sampai rata-rata tahunan) dibahas di bab keluaran; bab ini hanya menyediakan `in_d`, `out_d`, `water`, `wat` dan `Recall::d` yang dibacanya.
