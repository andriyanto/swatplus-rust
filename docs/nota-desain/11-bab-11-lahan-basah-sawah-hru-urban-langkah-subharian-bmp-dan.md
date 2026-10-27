## Bab 11  Lahan basah, sawah, HRU urban, langkah subharian, BMP, dan septik

Bab ini membahas modul-modul HRU yang "menyimpang" dari alur harian standar (bilangan kurva, MUSLE, perkolasi biasa): lahan basah di dalam HRU (*surface storage*, `surf_stor` pada `hru-data.hru`) termasuk sawah padi yang airnya ditahan pematang/ambang (*weir*); langkah waktu subharian dengan infiltrasi Green dan Ampt serta erosi percikan dan aliran permukaan per langkah; HRU urban dengan penumpukan dan pencucian padatan (*build-up/wash-off*) atau regresi USGS; praktik konservasi (BMP) di tepi dan di dalam lahan, yaitu jalur filter vegetatif, saluran berumput, dan efisiensi buatan pengguna; serta sistem septik dengan lapisan biozona yang disisipkan ke profil tanah.

File Rust yang dicakup seluruhnya: `crates/swat-engine/src/hru/wetland.rs`, `hru/subdaily.rs`, `hru/bmp.rs`, dan `hru/septic.rs`; ditambah dua method urban `hru_urban` dan `hru_urbanhr` beserta struct `Urban`/`UrbanRunoff` yang tinggal di `hru/mod.rs`. Dokumen pendamping di repo adalah `docs/subdaily.md`, `docs/paddy.md`, `docs/structures.md`, serta bagian "Wetland HRUs", "Rice paddies" dan "Penman-Monteith PET and urban HRUs" di `docs/swat-quirks.md`. Padanan Fortran utamanya: `wet_initial.f90`, `wetland_control.f90`, `res_hydro.f90`, `conditions.f90`, `res_weir_release.f90`, `swr_satexcess.f90`, `soils_init.f90`, `sq_greenampt.f90`, `ero_ovrsed.f90`, `sq_surfst.f90`, `stor_surfstor.f90`, `hru_urban.f90`, `hru_urbanhr.f90`, `regres.f90`, `structure_init.f90`, `structure_set_parms.f90`, `ttcoef_wway.f90`, `smp_filter.f90`, `smp_grass_wway.f90`, `smp_bmpfixed.f90`, `layersplit.f90` dan `sep_biozone.f90`.

Posisi dalam alur simulasi: semua fungsi di sini dipanggil dari kontrol HRU harian (`hru_control`, porting di `hru/mod.rs`). Lahan basah menggantikan perhitungan limpasan permukaan HRU (bilangan kurva tidak dihitung bagi HRU lahan basah); Green dan Ampt menggantikan bilangan kurva bila `time.sim` memiliki langkah > 1 dan `codes.bsn` `gampt` = 1; fungsi urban memodifikasi sedimen dan hara permukaan setelah erosi; BMP bekerja pada konstituen permukaan yang sudah ditunda (*lagged*) oleh `swr_substor`/`stor_surfstor`; dan septik berjalan setiap hari mengubah sifat lapisan biozona sebelum perkolasi. Seluruh fungsi mempertahankan perilaku SWAT+ commit 6f51536 bit demi bit, termasuk kekeliruannya; koreksi yang tersedia dengan `--fix-swat` disebutkan di entri terkait.

### 11.1  Lahan basah dan sawah: data, inisialisasi, dan ambang (`hru/wetland.rs`, bagian 1)

Lahan basah HRU adalah tampungan air di atas permukaan tanah HRU. Airnya bertambah dari hujan dan irigasi, merembes (*seepage*) ke tanah, menguap (dihitung di `et_act`, bab lain), dan dilepas menurut tabel keputusan `res_rel.dtl` (seperti waduk) atau, untuk sawah, melimpas lewat ambang (*weir*). Sedimen dan hara dalam air diendapkan dengan rutin waduk (`res_sediment`, `res_nutrient` dari modul `waterbody`). Sawah dikenali dari nama hidrologinya: baris `hydrology.wet` bernama `paddy`.

**Struct dan enum privat pendukung**

| Tipe | Field / varian | Arti dan satuan |
| --- | --- | --- |
| `Cond` | `evol: bool`, `op: char`, `lim_const: f32`, `alt: Vec<String>` | Satu kondisi tabel pelepasan: air tergenang (`vol_wet`, m3) dibandingkan dengan volume utama (`pvol`) atau darurat (`evol`) yang dikenai operator `*`, `+`, `-`, `/` dengan konstanta; `alt` adalah kolom alternatif (`<`, `>`, `=`, `-`) per alternatif. |
| `Release` | `Rate(m3/s)`, `RatePct(%)`, `AbEmer`, `Days{days, base, frac}`, `Dyrt{days, base, pct}` | Cara aksi melepas air (`res_hydro`): laju tetap, persen volume utama, kelebihan di atas volume darurat, penurunan ke volume dasar selama n hari, atau penurunan ditambah persen volume utama. |
| `Base` | `Zero`, `Pvol`, `Evol` | Volume dasar aksi `days`/`dyrt` (penunjuk file `null`, `pvol`, `evol`). |
| `Act` | `release: Release`, `outcomes: Vec<String>` | Satu aksi beserta kolom hasil (`y`/`n`) per alternatif. |

**Struct publik `Wetland`** (padanan gabungan `wet(j)`, `wet_ob(j)`, `wet_hyd(j)`, `wet_prm(j)`, `wet_seep_day(j)`):

| Field | Arti | Satuan |
| --- | --- | --- |
| `acoef`, `bcoef`, `ccoef` | Koefisien kurva volume–luas dan volume–kedalaman dari `hydrology.wet` | – |
| `wet_hc` | Konduktivitas hidraulik dasar genangan (`k`) | mm/jam |
| `paddy` | Benar bila hidrologi bernama `paddy` | – |
| `weir` | `Some((c, k, w))`: koefisien debit, eksponen, lebar ambang dari baris pertama `weir.res`; `None` tanpa file | – |
| `depth` | Kedalaman air tergenang seperti dilihat tabel keputusan (`wet_ob%depth`) | m |
| `evrsv`, `evrsv_et` | Koefisien evaporasi milik lahan basah, dan yang dipakai `et_act` (lihat quirk) | – |
| `pvol`, `evol` | Volume utama dan darurat | m3 |
| `weir_hgt` | Tinggi ambang | m |
| `alts`, `conds`, `acts` | Jumlah alternatif, kondisi, aksi tabel pelepasan | – |
| `prm` | Parameter `sediment.res` dan `nutrients.res` (`WbPrm`) | – |
| `water` | Air tergenang dan muatannya (`Hyd`: flo m3, sed t, hara kg) | m3, t, kg |
| `out` | Aliran keluar hari ini (`ht2`) | m3, t, kg |
| `seep_day` | Hara yang ikut merembes hari ini: no3, nh3, orgn, solp, sedp | kg |
| `water_seep` | Kedalaman rembesan hari ini (`hru%water_seep`) | mm |

#### 11.1.1  `find`

- **Lokasi:** `crates/swat-engine/src/hru/wetland.rs`, `fn find<'a>(t: Option<&'a Table>, field: &str, name: &str) -> Option<(&'a Table, usize)>` (privat).
- **Padanan SWAT+:** tidak ada; khusus port (menggantikan pencarian nama berulang di `wet_read`, `wet_read_hyd`, `wet_initial`).
- **Tujuan:** Mencari baris tabel input yang kolom `field`-nya sama dengan `name`.
- **Masukan dan keluaran:** tabel opsional, nama kolom, nama yang dicari; mengembalikan pasangan (tabel, indeks baris berbasis 0) atau `None` bila tabel tidak ada atau nama tidak ditemukan.
- **Cara kerja:**
  1. Bila tabel `None`, kembalikan `None`.
  2. Telusuri baris dari atas; baris pertama yang teksnya cocok menurut `fortran_eq` (perbandingan gaya Fortran, yakni mengabaikan spasi di belakang) dipilih.
- **Kebiasaan SWAT+ / catatan port:** baris pertama yang cocok yang diambil, sama seperti loop pencarian SWAT+ yang berhenti di kecocokan pertama.

#### 11.1.2  `Wetland::new`

- **Lokasi:** `hru/wetland.rs`, `pub fn new(project: &Project, name: &str, area_ha: f32) -> Result<(Wetland, usize), String>`.
- **Padanan SWAT+:** `wet_initial` (`wet_initial.f90`), bersama pembacaan `wet_read`, `wet_read_hyd`, `res_read_weir`, dan penunjuk tabel keputusan.
- **Tujuan:** Membangun lahan basah sebuah HRU seluas `area_ha` yang `surf_stor`-nya bernama `name`, dan mengembalikan pula nomor baris `hydrology.wet` berbasis 1 (`wet_dat%hyd`) yang diperlukan `et_act`.
- **Masukan dan keluaran:** tabel `wetland.wet`, `hydrology.wet`, `initial.res`, `om_water.ini`, `res_rel.dtl`, `weir.res`, `sediment.res`, `nutrients.res`; luas HRU (ha). Keluaran: `Wetland` siap pakai dengan air awal nol, atau pesan galat.
- **Cara kerja:**
  1. Cari baris `wetland.wet` bernama `name`; ambil nama hidrologi (`hyd`), lalu baris `hydrology.wet`-nya. Baca `psa`, `pdep`, `esa`, `edep`, `k`, `evrsv` (dan `acoef`, `bcoef`, `ccoef`).
  2. Tolak bila `psa`, `esa`, `evrsv` atau `k` ≤ 0: di SWAT+ `wet_read_hyd` mengisi nilai pengganti dari larik yang belum disetel, sehingga perilakunya tak terdefinisi.
  3. Air awal: bila baris `initial.res` menunjuk baris `om_water.ini` yang memuat nilai bukan nol, tolak (port hanya mendukung lahan basah yang mulai kosong).
  4. Tabel pelepasan: cari `res_rel.dtl` bernama `rel`. Untuk sawah, tabel yang hilang diganti tabel kosong dan kondisi/aksinya tidak dibaca (tabel sawah tidak dipakai).
  5. Setiap kondisi wajib berbentuk `vol_wet` pada objek 0 dengan batas `pvol` atau `evol` dan operator `* + - /`; selain itu ditolak.
  6. Setiap aksi diterjemahkan dari `option`: `rate` → `Rate(const)`, `rate_pct` → `RatePct(const)`, `ab_emer`, `days` → `Days{const, basis, const2}`, `dyrt` → `Dyrt{const, basis, const2}`; basis dari `file_pointer` (`null`, `pvol`, `evol`).
  7. Volume awal dari kedalaman (mm) dan luas (ha):

```latex
V_p = A\,d_p\,10, \qquad V_e = A\,d_e\,10, \qquad h_{weir} = d_p/1000
```

8. Bila `weir.res` ada dan tidak kosong, baris pertamanya menetapkan `h_weir = h` (m), parameter ambang `(c, k, w)`, dan

```latex
V_p = A\,h\,10^4, \qquad V_e \leftarrow \max(V_e,\ 1{,}2\,V_p)
```

9. Bentuk `WbPrm` dari nama `sed` dan `nut`, set `evrsv_et = evrsv`, kembalikan `(w, ih + 1)`.

- **Kebiasaan SWAT+ / catatan port:**
  - Ambang selalu dari **baris pertama `weir.res`** (`wet_ob%iweir` diawali 1); kolom `WEIR` di `wetland.wet` tidak dibaca. Hal ini berlaku bagi semua lahan basah, bukan hanya sawah (`docs/swat-quirks.md`, "Rice paddies"; `docs/paddy.md`).
  - `evrsv_et` mula-mula sama dengan `evrsv`, tetapi lapisan pemanggil menimpanya dengan nilai milik HRU yang bernomor sama dengan baris `hydrology.wet` (0,7 bila HRU itu tidak berlahan basah). Inilah bug `et_act` yang dikoreksi `--fix-swat` (memakai `evrsv` milik sendiri).
  - Laju pengendapan `nutrients.res` dibagi 365 di `res_read_nut` (tahunan ke harian), ditangani di `WbPrm::new`. Bila baris `sediment.res` lahan basah tidak ditemukan, `sed_stlr_co`, `soln_stl_fr` dan `solp_stl_fr` tetap 0 seperti di `wet_initial` (diperbaiki di 1.0.1, sesuai SWAT+; lihat 13.5.2).

#### 11.1.3  `Wetland::set_weir`

- **Lokasi:** `hru/wetland.rs`, `pub fn set_weir(&mut self, h_mm: f32, area_ha: f32)`.
- **Padanan SWAT+:** aksi tabel keputusan `weir_height` di `actions.f90` (cabang lahan basah).
- **Tujuan:** Menaikkan atau menurunkan ambang sawah/lahan basah dari tabel keputusan manajemen.
- **Masukan dan keluaran:** tinggi ambang baru (mm), luas HRU (ha); memperbarui `weir_hgt` (m), `pvol`, `evol` (m3).
- **Cara kerja:**
  1. `weir_hgt = h_mm / 1000`.
  2. `pvol = A · weir_hgt · 10000`.
  3. Bila `evol < 1,2 · pvol`, naikkan `evol` menjadi `1,2 · pvol`.
- **Kebiasaan SWAT+ / catatan port:** penunjuk file aksi diabaikan; `evol` hanya pernah dinaikkan, tidak diturunkan ketika ambang direndahkan (sesuai kode Rust: tidak ada cabang penurunan).

### 11.2  Lahan basah dan sawah: pelepasan, neraca harian, dan kelebihan jenuh (`hru/wetland.rs`, bagian 2)

Subbab ini memuat inti neraca air lahan basah. Urutan harian (dalam `wetland_control`) adalah: tambah hujan dan irigasi → hitung luas genangan → rembesan dibatasi kapasitas profil tanah → hara ikut rembesan ke lapisan atas → pelepasan (ambang sawah atau tabel keputusan) → pengendapan sedimen dan hara → penggelontoran (*flush*) bila genangan semula kosong. Kelebihan jenuh lapisan atas yang muncul kemudian di perkolasi dikembalikan ke genangan oleh `wet_excess`.

#### 11.2.1  `Wetland::weir_release`

- **Lokasi:** `hru/wetland.rs`, `fn weir_release(&mut self, area_ha: f32, dep: f32)` (privat).
- **Padanan SWAT+:** `res_weir_release` (`res_weir_release.f90`).
- **Tujuan:** Menghitung air yang keluar dari sawah: limpahan di atas volume darurat, lalu aliran di atas ambang dalam 24 langkah per jam.
- **Masukan dan keluaran:** luas HRU `A` (ha), kedalaman air `dep` (m); membaca `water.flo` (m3), `evol`, `weir_hgt`, `weir = (c, k, w)`. Menambah `out.flo` (m3) dan memperbarui `water.flo`.
- **Cara kerja:**
  1. `vol = water.flo`, luas permukaan `wsa1 = A · 10^4` m2, tinggi di atas ambang `h_a = max(dep − h_weir, 0)`.
  2. Bila tidak ada `weir.res` atau `h_a = 0`: `out.flo = 0`, selesai (tanpa mengubah `water.flo`).
  3. Bila `vol > evol`: limpahkan `water.flo − evol` ke `out.flo` (dijaga ≥ 0), `vol = evol`, dan hitung ulang `h_a = max(vol/wsa1 − h_weir, 0)`.
  4. Volume di atas ambang `V_a = h_a · wsa1`.
  5. Ulangi paling banyak 24 kali (satu jam per langkah):

```latex
q = (A \cdot 0{,}45)\; c\; w\; h_a^{\,k} \cdot 3600 \quad [\mathrm{m^3/jam}]
```

Bila `q > V_a`, keluarkan `V_a` (seluruh sisa di atas ambang); selain itu keluarkan `q`; kurangi `vol` dan `V_a`. Hitung ulang `h_a = max(vol/wsa1 − h_weir, 0)` (`vol/wsa1` diganti 0 bila `wsa1 ≤ 10^{-6}`). Berhenti bila `V_a ≤ 0,001` m3 atau `h_a ≤ 0,0001` m.

6. `water.flo = vol`.

- **Kebiasaan SWAT+ / catatan port:** pangkat memakai `rpow` (pangkat real gaya Fortran) agar bit demi bit. Faktor `A · 0,45` (luas dalam ha, bukan panjang) adalah rumus SWAT+ apa adanya. Tanpa ambang, atau dengan tinggi 0 di bawah air, tidak ada yang keluar; tabel pelepasan sawah tidak dipakai (`docs/swat-quirks.md`). Pelepasan di sini tidak dibagi menurut langkah subharian; pada mode subharian limpasan harian dibagi rata ke langkah oleh pemanggil.

#### 11.2.2  `Wetland::release`

- **Lokasi:** `hru/wetland.rs`, `fn release(&self, nstep: usize) -> f32` (privat).
- **Padanan SWAT+:** `conditions` (`conditions.f90`) dan `res_hydro` (`res_hydro.f90`).
- **Tujuan:** Mengevaluasi tabel keputusan pelepasan terhadap volume tergenang dan menjumlahkan aksi yang terpicu.
- **Masukan dan keluaran:** jumlah langkah waktu per hari `nstep`; membaca `water.flo` (m3), `pvol`, `evol`, kondisi dan aksi. Keluaran: volume yang dilepas hari ini (m3).
- **Cara kerja:**
  1. Semua alternatif mula-mula benar. Untuk setiap kondisi, nilai pembanding tabel `T = V op c` dengan `V` = `pvol` atau `evol`; `cond_test` (dari `hru/mgt.rs`, bab manajemen) menggugurkan alternatif yang kolomnya tidak dipenuhi oleh `cur = water.flo` terhadap `T`.
  2. Ulangi daftar aksi sebanyak `nstep` kali (iterator `cycle().take(len · nstep)`); aksi dijalankan bila tabel tanpa alternatif, atau ada alternatif benar yang hasilnya `y`.
  3. Kontribusi tiap aksi (dengan `ns = nstep`, `cur` volume awal, tidak diperbarui di dalam loop):

```latex
\begin{aligned}
\texttt{rate}:&\ Q \mathrel{+}= r \cdot 86400\\
\texttt{rate\_pct}:&\ Q \mathrel{+}= p\,V_p/100\\
\texttt{ab\_emer}:&\ Q \mathrel{+}= \max(cur - V_e, 0)\\
\texttt{days}:&\ Q \mathrel{+}= (cur - b_{lo})/n_d/ns,\ \ b_{lo} = f\cdot V_{basis}\ (0\ \text{bila null}),\ Q\leftarrow\max(Q,0)\\
\texttt{dyrt}:&\ Q \mathrel{+}= (cur - b_{lo})/n_d + p\,V_p/100/ns,\ \ b_{lo} = \max(V_{basis},0),\ Q\leftarrow\max(Q,0)
\end{aligned}
```

4. Kembalikan `Q`.

- **Kebiasaan SWAT+ / catatan port:**
  - Setiap aksi dijalankan **sekali per langkah waktu**: laju tetap, persen volume utama, dan kelebihan di atas volume darurat dilepas `nstep` kali sehari, sedangkan penurunan `days` dibagi `nstep`, dan suku pertama `dyrt` tidak dibagi (`docs/subdaily.md`, Quirks). Pada simulasi harian `nstep = 1` sehingga perilaku ini tak tampak.
  - Volume pembanding `cur` tidak dikurangi pelepasan di dalam loop, sesuai `res_hydro` yang membaca `wbody%flo` tetap.
  - Opsi `res_hydro` lain (`inflo_rate`, `inflo_frac`, `dyrt1`, dsb.) ditolak di `Wetland::new`.
  - Pembagian `/` pada kondisi tidak dijaga terhadap konstanta 0, sama seperti Fortran.

#### 11.2.3  `Hru::wetland_control`

- **Lokasi:** `hru/wetland.rs`, `pub(super) fn wetland_control(&mut self, precip: f32, mo: i32, station_tave: f32, nut: &NutBasin, nd: &mut NutDay) -> f32`.
- **Padanan SWAT+:** `wetland_control` (`wetland_control.f90`), memanggil `res_weir_release` atau `conditions` + `res_hydro`, lalu `res_sediment`, `res_nutrient`, `ero_cfactor`.
- **Tujuan:** Simulasi harian lahan basah HRU: masukan air, rembesan, pelepasan, pengendapan, dan muatan sedimen/hara yang meninggalkan HRU.
- **Masukan dan keluaran:** curah hujan efektif `precip` (mm), bulan `mo`, suhu rata-rata stasiun (°C), parameter basin (`prf`, `rsd_covco`), irigasi hari ini `irr_applied` (mm), lapisan tanah (`st`, `ul`, `k`, mm dan mm/jam). Keluaran: limpasan permukaan HRU = pelepasan (mm); `nd.sedyld` (t), `nd.sedorgn`, `nd.sedorgp`, `nd.surqno3`, `nd.sedminps`, `nd.sedminpa` (kg/ha); `no3`, `nh4`, P aktif lapisan 1 (kg/ha).
- **Cara kerja:**
  1. Siapkan `wsa1 = A · 10` (konversi mm ke m3), `nstep`, faktor C hari ini, `usle_mult`, `tconc`, dan **salinan** air tanah per lapisan (`st`, `ul`, `k`). Bila HRU tak berlahan basah, kembalikan 0.
  2. Nolkan `out`, `water_seep`; catat kedalaman awal `dep_init = water.flo/wsa1/1000` (m).
  3. `water.flo += precip · wsa1 + irr · wsa1`.
  4. Bila ada air: selesaikan kuadrat kurva volume–kedalaman,

```latex
x_1 = b^2 + 4c\left(1 - \frac{V}{V_p + 10^{-9}}\right),\qquad h = \begin{cases}0 & x_1 < 10^{-6}\\ \dfrac{-b-\sqrt{x_1}}{2c+10^{-9}} + b & \text{lainnya}\end{cases}
```

Lalu fraksi luas `f = clamp(1 + a · h, 0,01, 1)`; luas genangan `A_w = A` untuk sawah (datar) atau `A · f` untuk lahan basah lain.

5. Rembesan potensial `S = min(V, k_w · 24 · A_w · 10)` (m3). Bila setara `S/wsa1 > 0,1` mm, uji kapasitas profil: turun lapis demi lapis, tambahkan rembesan ke `st`; bila melampaui `0,999 · ul`, kelebihan diteruskan ke bawah paling banyak `k · 24` mm, selain itu rembesan berhenti. Lalu naik dari lapisan terbawah: kelebihan di atas `ul` dibawa ke atas sebagai `volex`. Rembesan dikurangi: `S = max(S − volex · wsa1, 0)`.
6. `water.flo −= S`, `water_seep = S/wsa1` (mm). Rasio rembesan `r_s = S/(S + V)` bila penyebut > 0,01, selain itu 0.
7. Hara ikut rembesan: `no3`, `nh4` dan P aktif lapisan 1 bertambah `water.{no3, nh3, solp} · r_s / A`; `seep_day` = no3, nh3, orgn, solp, sedp air × `r_s` (kg), dan dikurangkan dari air.
8. Pelepasan: sawah → `weir_release(A, V/wsa1/1000)`; lainnya → `out.flo = release(nstep)`, `water.flo = max(water.flo − out.flo, 0)`.
9. Limpasan `surfq = out.flo / wsa1` (mm); `depth = water.flo/wsa1/1000` (m).
10. `res_sediment` dan `res_nutrient` (bulan, suhu stasiun) mengendapkan dan membagi muatan antara `water` dan `out`.
11. Penggelontoran: bila `dep_init < 0,0001` m dan ada aliran keluar, sedimen dihitung ulang dengan bentuk MUSLE:

```latex
q_p = \frac{prf}{6578{,}6}\,\frac{A\,Q}{t_c}\,\frac{1}{35{,}3},\qquad sed = (10\,Q\,q_p\,A)^{0{,}56}\, C\, \mathrm{usle\_mult},\qquad sedyld = sed\,\frac{Q_{out}}{V + Q_{out}}
```

Kemudian `water.sed = max(sed − sedyld, 0)`, `out.sed = sedyld`, dan `usle_cfac` HRU disetel ke faktor C hari ini.

12. Muatan: `nd.sedyld = out.sed + sedyld`; `sedorgn = out.orgn/A`, `sedorgp = out.sedp/A`, `surqno3 = out.no3/A`, `sedminps = sedminpa = out.solp/A/2`.
13. Tulis kembali hara lapisan 1; kembalikan `surfq`.

- **Kebiasaan SWAT+ / catatan port:**
  - **Uji rembesan memakai salinan air tanah** (`swst` di Fortran): ia membatasi rembesan pada yang dapat ditampung profil, tetapi tanah sendiri baru terisi pada perkolasi berikutnya (`docs/swat-quirks.md`).
  - **Sedimen terhitung dua kali saat genangan kosong melimpas**: sedimen gelontoran menjadi milik `out`, lalu ditambahkan lagi ke `sedyld` yang sudah memuatnya.
  - Bilangan kurva tidak dihitung bagi HRU lahan basah, sehingga `hru_wb_day` mencetak `cn` 0; `nut_orgn` dan `nut_psed` tidak berjalan (perlu `qp_cms` yang hanya disetel `surface`), sehingga N organik dan P sedimen HRU adalah yang dilepas lahan basah.
  - Fortran juga menambahkan N dan P organik air ke pool `soil1%water(1)` dan nitrat irigasi (`irrig%no3`) ke genangan; langkah ini tidak ada dalam fungsi Rust ini, tetapi sudah diperiksa dan setara. Kolam `soil1%water` hanya dipindahkan oleh pencampuran tanah dan tidak dibaca proses apa pun, jadi tidak ada keluaran yang bergantung padanya; `irrig%no3` hanya diisi oleh operasi jadwal `irrp`, yang ditolak port. Komentar tentang hal ini kini ada di `wetland.rs`.
  - Mode `gwflow` (`gwflow_wetland`) tidak didukung; hanya rembesan asli.

#### 11.2.4  `Hru::wet_excess`

- **Lokasi:** `hru/wetland.rs`, `pub(super) fn wet_excess(&mut self, ul_excess: f32, surfq: &mut f32)`.
- **Padanan SWAT+:** cabang lahan basah di `swr_satexcess` (`swr_satexcess.f90`).
- **Tujuan:** Mengembalikan air yang tak tertampung lapisan atas yang jenuh ke genangan, beserta hara yang tadi ikut merembes.
- **Masukan dan keluaran:** kelebihan jenuh `ul_excess` (mm), limpasan `surfq` (mm, dapat bertambah). Memperbarui `water`, `out`, `water_seep`, `depth`, dan hara lapisan 1.
- **Cara kerja:**
  1. Bila tinggi ambang < 0,001 m: kelebihan menjadi aliran keluar (`out.flo += ul_excess · 10 · A`) dan `surfq += ul_excess`. Selain itu kembali ke genangan (`water.flo += ul_excess · 10 · A`).
  2. `water_seep = max(water_seep − ul_excess, 0)`; `depth = water.flo / A / 10000` (m).
  3. Bila `water_seep > 10^{-6}`: `rto = min(ul_excess / water_seep, 1)`; kurangi no3, nh4, P aktif lapisan 1 sebesar `seep_day · rto / A` dan tambahkan no3, nh3, orgn, solp, sedp `seep_day · rto` ke air.
- **Kebiasaan SWAT+ / catatan port:** `rto` dihitung dengan `water_seep` yang **sudah** dikurangi, sesuai urutan Fortran. Limpasan tambahan dari kelebihan jenuh ini tidak masuk hidrograf subharian (lihat 11.3).

### 11.3  Langkah subharian: Green dan Ampt, erosi per langkah, dan tunda (`hru/subdaily.rs`)

Bila `time.sim` menetapkan langkah > 1 per hari, SWAT+ menjalankan lahan dan saluran per langkah (`time%step` langkah, panjang `time%dtm = 1440/step` menit). Dengan `codes.bsn` `gampt` = 1, infiltrasi Green dan Ampt menggantikan bilangan kurva, erosi dihitung per langkah dengan model percikan dan aliran permukaan, lalu limpasan dan sedimen ditunda per langkah. Dengan `gampt` = 0, HRU tetap memakai limpasan bilangan kurva harian yang kemudian disebar oleh hidrograf satuan (dibahas di bab perutean; `docs/subdaily.md`). Port mendukung hingga 96 langkah per hari (15 menit), konstanta `MAX_STEP = 96`.

**Struct `SubDaily`** (state subharian per HRU):

| Field | Arti | Satuan |
| --- | --- | --- |
| `step`, `dtm` | Langkah per hari dan panjangnya (`time%dtm`) | –, menit |
| `ga` | Green dan Ampt aktif (`gampt` 1) | – |
| `wfsh` | Hisapan front pembasahan lapisan atas | mm |
| `cvm_com` | Rata-rata ln faktor C minimum tanaman komunitas | – |
| `swtrg`, `rateinf_prev` | Penanda hari basah (> 12 mm) yang laju infiltrasinya dibawa ke hari berikut | –, mm/jam |
| `urb_abstinit` | Abstraksi awal yang tersisa di area kedap air | mm |
| `bs_q`, `bs_sed` | Limpasan dan sedimen yang tertahan di akhir tiap langkah (`hhsurf_bs(1:2, j, :)`) | mm, t |

**Struct `SubBasin`** (parameter basin yang dipakai langkah subharian): `cn_froz` (koefisien tanah beku), `urb_init_abst` (abstraksi awal urban maksimum, mm), `eros_spl` (koefisien erosi percikan, `splash`), `rill_mult` (pengali alur, `rill`), `eros_expo` (eksponen tegangan geser, `surq_exp`), `c_factor` (`cov_mgt`), semuanya dari `parameters.bsn`.

#### 11.3.1  `wfsh`

- **Lokasi:** `hru/subdaily.rs`, `pub fn wfsh(por: f32, clay: f32, sand: f32) -> f32`.
- **Padanan SWAT+:** perhitungan `wfsh` di `soils_init` (`soils_init.f90`).
- **Tujuan:** Menghitung hisapan kapiler di front pembasahan (*wetting front suction*) untuk Green dan Ampt dari sifat lapisan tanah atas.
- **Masukan dan keluaran:** porositas `φ` (fraksi), liat `C` dan pasir `S` (%); keluaran hisapan (mm).
- **Cara kerja:** persamaan regresi Rawls dan Brakensiek:

```latex
\begin{aligned}
x =\ & 6{,}5309 - 7{,}32561\,\phi + 3{,}809479\,\phi^2 + 0{,}001583\,C^2 + 0{,}000344\,S C - 0{,}049837\,\phi S\\
& + 0{,}001608\,\phi^2 S^2 + 0{,}001602\,\phi^2 C^2 - 0{,}0000136\,S^2 C - 0{,}003479\,C^2 \phi - 0{,}000799\,S^2 \phi\\
\psi_{wf} =\ & 10\,e^{x}
\end{aligned}
```

- **Kebiasaan SWAT+ / catatan port:** pasir yang dipakai adalah `100 − liat − debu` seperti yang ditinggalkan `soil_phys_init` untuk mode subharian (bukan kolom pasir input). Urutan suku penjumlahan dipertahankan persis demi kesamaan bit f32. Faktor 10 mengubah cm ke mm.

#### 11.3.2  `Hru::green_ampt`

- **Lokasi:** `hru/subdaily.rs`, `pub(super) fn green_ampt(&mut self, ts: &[f32], sta_ts: &[f32], cnday: &mut f32, pet_day: f32, b: &SubBasin, ubnrunoff: &mut [f32], hhqday: &mut [f32], hhsurfq: &mut [f32]) -> f32`.
- **Padanan SWAT+:** `sq_greenampt` (`sq_greenampt.f90`).
- **Tujuan:** Menghitung limpasan permukaan per langkah dengan infiltrasi Green dan Ampt (versi Mein dan Larson), termasuk limpasan area kedap air terhubung langsung pada HRU urban, dan memperbarui bilangan kurva tiap langkah.
- **Masukan dan keluaran:** hujan per langkah setelah intersepsi kanopi `ts` (mm), hujan stasiun per langkah `sta_ts` (mm, untuk area kedap air), `cnday`, PET harian (mm), parameter basin; lapisan 1 (`k` mm/jam, `por`), `sw`, `sumfc` (mm), `smx`, `wrt`. Keluaran: `hhqday` (limpasan pervious per langkah, mm), `ubnrunoff` (limpasan kedap air per langkah, mm), `hhsurfq` (jumlahnya), `cnday` diperbarui; nilai kembali = total limpasan harian (mm).
- **Cara kerja:**
  1. Siapkan larik kumulatif berindeks 0..n+1: hujan kumulatif `cumr`, infiltrasi kumulatif `cuminf`, kelebihan kumulatif `excum`, kelebihan per langkah `exinc`, laju infiltrasi `rateinf`, intensitas `rintns`.
  2. Defisit kelembapan front: bila `swtrg` (kemarin hari basah) maka `Δθ = 0,001 · φ · 0,95` dan `rateinf[1] = rateinf_prev` (lalu penanda dihapus); selain itu `sw` dibatasi `0,999 · sumfc` bila ≥ `sumfc`, `Δθ = (1 − sw/sumfc) · φ · 0,95` dan `rateinf[1] = 2000` mm/jam. `ψΔθ = Δθ · wfsh`.
  3. Untuk tiap langkah `k = 1..n`: konduktivitas efektif yang disesuaikan bilangan kurva (Nearing dkk.)

```latex
K_e = \frac{56{,}82\,K_{sat}^{0{,}286}}{1 + 0{,}051\,e^{0{,}062\,CN}} - 2,\qquad K_e \leftarrow 0{,}001\ \text{bila}\ K_e \le 0
```

4. `cumr[k] = cumr[k−1] + ts[k]`, intensitas `i_k = 60 · ts[k] / dtm` (mm/jam).
5. Bila `rateinf[k] ≥ i_k`: semua hujan terinfiltrasi (`cuminf[k] = cuminf[k−1] + ts[k]`, `excum` tetap). Selain itu, cari `F_k` dengan iterasi titik tetap mulai `F = K_e · dtm/60` hingga |beda| ≤ 0,001:

```latex
F_k = F_{k-1} + K_e\frac{\Delta t}{60} + \psi\Delta\theta\,\ln\frac{F + \psi\Delta\theta}{F_{k-1} + \psi\Delta\theta}
```

6. Lalu `excum[k] = max(cumr[k] − F_k, 0)`, `exinc[k] = max(excum[k] − excum[k−1], 0)`, `hhqday[k] = exinc[k]`.
7. HRU urban: abstraksi awal berkurang oleh hujan bila `i_k > 0,017` mm/jam, selain itu pulih `min(urb_init_abst, abst + PET/n)`. Limpasan pervious dikalikan `1 − fcimp`; limpasan kedap air `ubnrunoff[k] = max((sta_ts[k] − abst) · fcimp, 0)`.
8. `hhsurfq[k] = hhqday[k] + ubnrunoff[k]`, dijumlahkan ke `surfq`.
9. Laju infiltrasi berikutnya `rateinf[k+1] = K_e (ψΔθ/(F_k + 10^{-6}) + 1)`.
10. Perbarui bilangan kurva dengan air tanah `sw + F_k`: `sw_fac = clamp(wrt0 − wrt1 · swdt, −20, 20)`, `r2 = smx (1 − swdt/(swdt + e^{sw_fac}))` (atau `smx` bila penyebut ≤ 0,001); bila suhu lapisan 2 ≤ 0 °C, `r2 = smx (1 − e^{−cn_froz · r2})`; `r2 ≥ 3`; `CN = 25400/(r2 + 254)`.
11. Setelah loop: bila total hujan langkah > 12 mm, set `swtrg` dan simpan `rateinf[n+1]` untuk esok.

- **Kebiasaan SWAT+ / catatan port:**
  - `ubnrunoff` adalah larik SWAT+ yang **dipakai bersama oleh semua HRU** dan hanya diisi HRU urban; port meneruskannya sebagai irisan bersama sehingga HRU non-urban membaca nilai yang ditinggalkan HRU urban sebelumnya, sama seperti Fortran.
  - Suhu lapisan 2 dipakai untuk uji tanah beku (0 bila hanya ada satu lapisan).
  - `hhqday[k]` hanya ditulis pada langkah yang tidak terinfiltrasi penuh; pada langkah lain nilainya dari pemanggil (biasanya nol).
  - Kanopi mengambil air dari langkah dua kali dan tidak dari hujan harian, sedangkan infiltrasi harian tetap menghitung hujan harian penuh; limpasan kelebihan jenuh yang ditambahkan setelah langkah permukaan tidak pernah masuk hidrograf (`docs/subdaily.md`, Quirks). Kedua perilaku ini ada di pemanggil, bukan di fungsi ini.

#### 11.3.3  `Hru::ero_ovrsed`

- **Lokasi:** `hru/subdaily.rs`, `pub(super) fn ero_ovrsed(&self, sta_ts: &[f32], hhqday: &[f32], b: &SubBasin) -> Vec<f32>`.
- **Padanan SWAT+:** `ero_ovrsed` (`ero_ovrsed.f90`); model percikan dari EUROSEM (Morgan) dan erosi alur/antar-alur dari ANSWERS termodifikasi (Park dkk. 1982), menurut komentar sumber Fortran.
- **Tujuan:** Menghitung hasil sedimen per langkah (t) dari percikan butir hujan dan aliran permukaan.
- **Masukan dan keluaran:** hujan stasiun per langkah (mm), limpasan per langkah sebelum tunda `hhqday` (mm), parameter basin; lapisan 1 (liat, debu, `usle_k`), `lai_sum`, `cht_mx` (tinggi kanopi maksimum, m), `km` (km2), `slope`, biomassa atas tanah dan residu (kg/ha), `fimp` urban. Keluaran: vektor sedimen per langkah (t).
- **Cara kerja:**
  1. Kepekaan lepas tanah `erod_k` (g/J) dipilih berurutan dari tekstur (pasir = 100 − liat − debu): 2,0; 1,7; 2,0; 1,6; 1,9; 1,6; 1,2; 1,5; 2,0; 2,1; 2,6; 3,0; atau 1,9, sesuai cabang `if` pertama yang terpenuhi (lihat kode).
  2. Tutupan kanopi `cc = min(lai_sum, 1)`.
  3. Untuk tiap langkah: intensitas `i = 60 · P_k/dtm` (mm/jam), diameter median butir `d50 = 0,188 · i^{0,182}`. Bila `i > 0`: energi kinetik langsung `KE_d = max(8,95 + 8,44 log10 i, 0)`, energi tetesan daun `KE_l = max(15,8 √(0,5 h_c) − 5,87, 0)`, kedalaman hujan `r = P_k/(60·dtm)`, dibagi ke daun `r·cc` dan langsung `r(1−cc)`.

```latex
KE = 0{,}001\,(r_d\,KE_d + r_l\,KE_l),\qquad sed_{spl} = k_{erod}\,KE\,e^{-\,eros\_spl\cdot q/1000}\,km
```

4. Kalikan `1 − fimp` untuk HRU urban; set 0 bila `q ≥ 3 · d50` atau `q ≤ 0,001` mm.
5. Faktor C dinamis dari tutupan `cov = biomassa + residu`: `C = exp((−0,2231 − cvm) e^{−0,00115 cov} + cvm)`.
6. Erosi aliran permukaan dari tegangan geser dasar:

```latex
\tau = 9807\,\frac{q}{1000}\,s,\qquad sed_{ov} = 16{,}667\,\Delta t\,km\cdot 11{,}02\; rill\; K_{usle}\; c_{bsn}\; C\; \tau^{\,eros\_expo}
```

7. Kalikan `1 − fimp` untuk urban; hasil langkah `sed_spl + sed_ov`, dinolkan bila < 10^{-10}.

- **Kebiasaan SWAT+ / catatan port:** nama `rdepth` dipertahankan meski dibagi `dtm · 60` (laju per detik). Sedimen MUSLE harian tetap menentukan hara, tetapi yang meninggalkan HRU adalah erosi per langkah ini; dengan bilangan kurva (`gampt` 0) fungsi ini tak dipanggil sehingga HRU tidak mengeluarkan sedimen sama sekali (`docs/subdaily.md`).

#### 11.3.4  `Hru::lag_sediment`

- **Lokasi:** `hru/subdaily.rs`, `pub(super) fn lag_sediment(&mut self, hhsedy: &mut [f32]) -> f32`.
- **Padanan SWAT+:** bagian per langkah dari `stor_surfstor` (`stor_surfstor.f90`).
- **Tujuan:** Menunda sedimen per langkah melalui tampungan permukaan dengan faktor tunda `brt`.
- **Masukan dan keluaran:** sedimen per langkah (t), diganti oleh yang keluar; `bs_sed` (t) state; nilai kembali total harian keluar (t).
- **Cara kerja:**
  1. `prev` = tampungan langkah terakhir hari sebelumnya `bs_sed[n−1]`.
  2. Untuk tiap langkah: `bs = max(prev + sed_k, 10^{-9})`, lalu bila tepat `10^{-9}` dijadikan 0; keluar `sed_k = bs · brt`; `bs −= sed_k`; `prev = bs`.
  3. Jumlahkan keluaran.
- **Kebiasaan SWAT+ / catatan port:** trik `max(…, 1e-9)` lalu uji kesamaan persis meniru Fortran. Dalam mode subharian tampungan sedimen harian mengambil hasil MUSLE sekali per langkah dan mengembalikan total langkah sekali (`docs/subdaily.md`); itu terjadi di pemanggil.

#### 11.3.5  `Hru::lag_runoff`

- **Lokasi:** `hru/subdaily.rs`, `pub(super) fn lag_runoff(&mut self, hhsurfq: &mut [f32]) -> f32`.
- **Padanan SWAT+:** cabang Green dan Ampt dari `sq_surfst` (`sq_surfst.f90`).
- **Tujuan:** Menunda limpasan per langkah dengan cara yang sama seperti sedimen.
- **Masukan dan keluaran:** limpasan per langkah (mm), diganti oleh yang keluar; state `bs_q` (mm); nilai kembali `qday` (mm).
- **Cara kerja:** identik dengan `lag_sediment` tetapi pada `bs_q`, sambil menjumlahkan `qday` di dalam loop:

```latex
B_k = \max(B_{k-1} + q_k, 10^{-9}),\quad q_k^{out} = brt\,B_k,\quad B_k \leftarrow B_k - q_k^{out}
```

- **Kebiasaan SWAT+ / catatan port:** `qday` harian adalah jumlah limpasan langkah yang sudah ditunda dan disetel sebelum perkolasi, sehingga limpasan kelebihan jenuh yang ditambahkan kemudian tidak masuk hidrograf. Penjumlahan di dalam loop (bukan setelahnya, beda dengan `lag_sediment`) mengikuti urutan Fortran demi kesamaan bit.

### 11.4  HRU urban: penumpukan dan pencucian padatan (`hru/mod.rs`)

HRU urban adalah HRU yang tata gunanya menunjuk baris `urban.urb`. Sebagian luasnya kedap air (`fimp`), sebagian dari itu terhubung langsung ke saluran (`fcimp`). Limpasan area kedap air memakai bilangan kurva 98 (harian, di rutin bilangan kurva; bab lain) atau, per langkah, hujan stasiun dikurangi abstraksi awal (11.3.2). Dua method di bawah mengganti bagian kedap air dari sedimen dan hara permukaan dengan muatan padatan yang tercuci. Keduanya tinggal di `hru/mod.rs` tetapi dibahas di sini karena terkait langsung dengan langkah subharian.

**Struct `Urban`** (`urban_db`): `fimp`, `fcimp` (fraksi HRU); `curbden` (kerapatan panjang trotoar, km/ha); `urbcoef` (koefisien pencucian, 1/mm); `dirtmx` (padatan maksimum, kg/km trotoar); `thalf` (hari hingga setengah maksimum); `tnconc`, `tpconc`, `tno3conc` (N total, P total, nitrat dalam padatan, mg/kg); `ro: UrbanRunoff`.

**Enum `UrbanRunoff`** (`urb_ro`): `BuildupWashoff` (penumpukan/pencucian), `UsgsReg(kategori)` (regresi USGS Driver dan Tasker, dengan kategori curah hujan tahunan generator cuaca 0: < 508 mm, 1: < 1.016 mm, 2: lebih), dan `None` (nilai lain: tanpa muatan dari area kedap air).

#### 11.4.1  `Hru::hru_urban`

- **Lokasi:** `crates/swat-engine/src/hru/mod.rs`, `fn hru_urban(&mut self, surfq: f32, phh: f32, precip: f32, nd: &mut NutDay) -> f32` (privat).
- **Padanan SWAT+:** `hru_urban` (`hru_urban.f90`) dan fungsi `regres` (`regres.f90`).
- **Tujuan:** Penumpukan dan pencucian padatan harian di area kedap air (atau beban regresi USGS), yang menggantikan bagian kedap air dari sedimen dan haranya.
- **Masukan dan keluaran:** limpasan harian `surfq` (mm), fraksi hujan setengah jam `phh` (alpha 0,5 h), hujan harian (mm), `nd.qp_cms` (m3/s), `km`, `tconc` (jam), `twash` (hari sejak cucian terakhir). Keluaran: `nd.sedyld`, `sedorgn`, `surqno3`, `sedorgp`, `surqsolp` diperbarui; nilai kembali = debu tambahan `silyld`.
- **Cara kerja (cabang `UsgsReg(ii)`):**
  1. Jalan hanya bila hujan > 0,1 mm dan limpasan > 0,1 mm.
  2. Untuk konstituen k (0 COD, tidak dipakai; 1 padatan tersuspensi; 2 N total; 3 P total) dengan koefisien `B[k][ii]`:

```latex
Y_k = \frac{\beta_0 \left(\frac{P}{25{,}4}\right)^{\beta_1} \left(\frac{km\,f_{imp}}{2{,}589}\right)^{\beta_2} (100 f_{imp} + 1)^{\beta_3}\,\beta_4}{2{,}205}\quad[\mathrm{kg}]
```

3. Dengan `rest = 1 − fimp`: `sedyld = 0,001 SS · fimp + sedyld · rest`; `silyld = 0,001 SS · fimp`; `sedorgn` dan `surqno3` mendapat 0,7 dan 0,3 dari `TN/(100 km)`, `sedorgp` dan `surqsolp` mendapat 0,75 dan 0,25 dari `TP/(100 km)`, masing-masing × `fimp` plus nilai lama × `rest`.

- **Cara kerja (cabang `BuildupWashoff`):**
  1. Bila `surfq ≤ 0,1` mm: `twash += 1` hari, selesai.
  2. Penumpukan: `dirt = dirtmx · twash/(thalf + twash)` (kg/km trotoar), simpan `dirto`.
  3. Koefisien pencucian `urbk = urbcoef · (qp_cms · 3,6 / km)` (laju limpasan puncak, mm/jam). Durasi hujan dari `phh`: `rp1 = −2 ln(1 − phh)`, `durf = 4,605/rp1`, `turo = min(durf + tconc, 24)`.
  4. `xx = min(urbk · turo, 24)`; `dirt = dirt · e^{−xx}` (0 bila < 10^{-6}).
  5. Waktu penumpukan setara sisa: `twash = thalf · dirt/(dirtmx − dirt)`.
  6. Padatan tercuci `SS = max((dirto − dirt) · curbden, 0)` (kg/ha); `TN = tnconc · SS/10^6`, `TP`, `TNO3` serupa.
  7. `sedyld = 0,001 SS · fimp + sedyld · rest`; `silyld = 0,001 SS · A · fimp`; `surqno3 = TNO3 · fimp + …`; `sedorgn = (TN − TNO3) · fimp + …`; `sedorgp = 0,75 TP · fimp + …`; `surqsolp = 0,25 TP · fimp + …`.
- **Kebiasaan SWAT+ / catatan port:**
  - **Debu urban dalam ton, bukan t/ha**: padatan dikalikan luas HRU untuk kelas debu tetapi tidak untuk sedimen (`docs/swat-quirks.md`).
  - Pilihan `urb_ro` selain dua cabang ini tidak menambah muatan.
  - `docs/swat-quirks.md` masih menyatakan `usgs_reg` "tidak diporting", tetapi kode versi 1.0 memuat cabang regresi lengkap dengan tabel koefisien `regres`; dokumen itu tampaknya tertinggal.

#### 11.4.2  `Hru::hru_urbanhr`

- **Lokasi:** `hru/mod.rs`, `fn hru_urbanhr(&mut self, surfq: f32, ubnrunoff: &[f32], nd: &mut NutDay)` (privat).
- **Padanan SWAT+:** `hru_urbanhr` (`hru_urbanhr.f90`).
- **Tujuan:** Versi per langkah dari penumpukan/pencucian: setiap langkah dengan limpasan kedap air yang cukup mencuci padatan dan mengganti bagian kedap air dari hara hari itu.
- **Masukan dan keluaran:** limpasan harian (mm), limpasan kedap air per langkah `ubnrunoff` (mm, dari 11.3.2); state `twash` (hari). Memperbarui `nd.surqno3`, `sedorgn`, `sedorgp`, `surqsolp` (kg/ha).
- **Cara kerja:** untuk tiap langkah k:
  1. Laju `q = ubnrunoff[k] · 60/dtm` (mm/jam).
  2. Bila `q > 0,025` dan `surfq > 0,1`: `dirto = dirtmx · twash/(thalf + twash)`; `urbk = urbcoef · q`; `dirt = dirto · e^{−urbk · dtm/60}` (dalam f64; 0 bila < 10^{-6}); `twash = thalf · dirt/(dirtmx − dirt)`; `SS = max((dirto − dirt) curbden, 0)`; `TN`, `TP` seperti di atas, dan `TNO3` dipotong ke bilangan bulat; lalu hara diganti dengan rumus bobot `fimp`/`rest` yang sama dengan harian.
  3. Selain itu `twash += dtm/1440` hari.
- **Kebiasaan SWAT+ / catatan port:**
  - **Per langkah padatan dicuci apa pun `urb_ro`-nya** (termasuk `usgs_reg`), dan setiap pencucian mengganti bagian kedap air dari hara harian **lagi**, di atas penggantian langkah sebelumnya. Sedimen tidak diganti.
  - **Nitrat `tno3` adalah integer** di Fortran: hanya kilogram per hektare utuh yang terhitung, sehingga dalam praktik bagian kedap air tidak bernitrat dan N organiknya adalah seluruh N total (`docs/subdaily.md`).
  - `dirt` berpresisi ganda di Fortran; port menghitung `dirt`, `twash` dan `SS` dalam f64 lalu membulatkan ke f32 di titik yang sama.
  - Validasi: Ps\_urbhr (Ps\_hourly dengan dua HRU lahan pertanian umum diganti kota kepadatan sedang `urmd`) cocok di semua tabel.

### 11.5  Praktik konservasi (BMP): jalur filter, saluran berumput, efisiensi pengguna (`hru/bmp.rs`)

Tiga praktik struktural mengurangi sedimen dan hara di limpasan permukaan **setelah** ditunda oleh `swr_substor`/`stor_surfstor`, dengan urutan: jalur filter (`smp_filter`), saluran berumput (`smp_grass_wway`), lalu efisiensi pengguna (`smp_bmpfixed`). Tidak satu pun mengubah volume air. Praktik dipilih dari baris `landuse.lum` (nama `filterstrip.str`, `grassedww.str`, `bmpuser.str`).

**Struct dan tipe data**

| Tipe | Field | Arti dan satuan |
| --- | --- | --- |
| `Filter` | `ratio`, `con`, `ch` | Rasio luas lahan terhadap jalur (`fld_vfs`), fraksi lahan yang mengalir ke 10 % jalur yang paling terkonsentrasi (`con_vfs`), fraksi aliran terkonsentrasi itu yang sepenuhnya tersalurkan (`cha_q`) |
| `GrassWw` | `n`, `spcon`, `d`, `w`, `l`, `s` | Manning n, koefisien sedimen (`sed_co`), kedalaman (m), lebar (m), panjang (km), kemiringan (m/m) |
|  | `tc` | Waktu konsentrasi saluran (jam, `tc_gwat`) |
|  | `wid_btm`, `area`, `vel_bf`, `celerity_bf` | Lebar dasar (m), luas penampang penuh (m2), debit penuh dari Manning (m3/s; nama field "vel" tetapi rumus `A·R^{2/3}·√s/n`), selerity penuh (m/s) |
| `BmpUser` | `sed`, `pp`, `sp`, `pn`, `sn` | Fraksi (0–1) yang dibuang: sedimen, P partikulat, P terlarut, N partikulat, N terlarut |
| `Bmps` | `filter`, `grassww`, `user` | Praktik satu HRU (masing-masing opsional) |
| `Surface<'a>` | `lagged: &mut [f32; 7]`, `cls: &mut [f32; 5]` | Konstituen permukaan tertunda: sedimen (t), N organik, P organik, nitrat, P terlarut, P mineral aktif dan stabil (kg/ha); kelas sedimen pasir, debu, liat, agregat kecil, agregat besar (t) |

**Fungsi pembantu kecil**

| Fungsi | Lokasi | Kegunaan |
| --- | --- | --- |
| `qman(a, rh, n, s) -> f32` | `hru/bmp.rs` (privat) | Debit Manning `a · rh^{0,6666} · √s / (n + 0,001)`; padanan fungsi `Qman` SWAT+. Eksponen 0,6666 (bukan 2/3) dan `+ 0,001` dipertahankan. |
| `find(t, name)` | `hru/bmp.rs` (privat) | Mencari baris tabel berdasarkan nama (kolom pertama); `None` bila tabel/nama tak ada. Khusus port. |
| `pct(v, hi) -> f32` | `hru/bmp.rs` (privat) | Membatasi persen ke `[0, hi]` dengan dua `if` berurutan (atas dahulu, lalu bawah), seperti di `smp_filter`. |

#### 11.5.1  `init`

- **Lokasi:** `hru/bmp.rs`, `pub fn init(project: &Project, names: [&str; 3], km: f32, slope: f32) -> Bmps`.
- **Padanan SWAT+:** `structure_init` (`structure_init.f90`), `structure_set_parms` (`structure_set_parms.f90`), `ttcoef_wway` (`ttcoef_wway.f90`), `scen_read_bmpuser`.
- **Tujuan:** Membaca praktik yang dinamai tata guna lahan HRU dan menyiapkan parameter turunannya.
- **Masukan dan keluaran:** nama `[filterstrip, grassedww, bmpuser]`, luas HRU (km2), kemiringan HRU; keluaran `Bmps`.
- **Cara kerja:**
  1. Jalur filter: hanya bila `flag > 0`. `ratio` dibatasi `[0, 300]`; `con` ≤ 0 → 0,5, lalu ≤ 0,1 → 0,1, > 0,95 → 0,95; `ch` dibatasi `[0, 0,95]`.
  2. Saluran berumput: nilai bawaan bila ≤ 0: `n = 0,35`, `l = √km`, `d = 3/64 · w`, `s = 0,75 · slope`, `spcon = 0,005`.
  3. Waktu konsentrasi saluran:

```latex
t_c = \frac{0{,}62\,L\,n^{0{,}6}}{km^{0{,}125}\,s^{0{,}375}}
```

4. Bila `w − 2 · 8 · d ≤ 0`, `d = 3/64 · w`. `ttcoef_wway` dengan kemiringan sisi 8: lebar dasar `bw = w − 2 d · 8`; bila ≤ 0, `bw = 0,5 w` dan kemiringan sisi dihitung ulang `(w − bw)/(2d)`. Keliling basah `p = bw + 2d√(z^2+1)`, luas `a = bw·d + z·d^2`, `rh = a/p`; `vel_bf = qman(a, rh, n, s)`, `celerity_bf = qman(1, rh, n, s) · 5/3`.
5. BMP pengguna: baca `sed_eff`, `ptlp_eff`, `solp_eff`, `ptln_eff`, `soln_eff`.

- **Kebiasaan SWAT+ / catatan port:**
  - Nama yang tidak ada di file memberi "tanpa praktik" (baris 0 tabel SWAT+ kosong).
  - **Jalur filter hanya bekerja bila `flag` = 1**, tetapi saluran berumput dan BMP pengguna bekerja apa pun nilai flag-nya.
  - **Waktu konsentrasi saluran berumput tanpa waktu aliran darat**: `structure_init` berjalan sebelum `time_conc_init` menghitung `t_ov`, sehingga yang dijumlahkan bernilai 0.

#### 11.5.2  `trap_classes`

- **Lokasi:** `hru/bmp.rs`, `fn trap_classes(cls: &mut [f32; 5], trap: f32)` (privat).
- **Padanan SWAT+:** blok pengurangan kelas sedimen di `smp_filter` dan `smp_grass_wway`.
- **Tujuan:** Mengambil sedimen yang terjebak (t) dari kelas-kelas sedimen menurut urutan prioritas.
- **Masukan dan keluaran:** kelas `[pasir, debu, liat, agregat kecil, agregat besar]` (t), jumlah terjebak (t).
- **Cara kerja:**
  1. Sisa `xrem = trap`; telusuri kelas dengan urutan agregat besar → pasir → agregat kecil → debu → liat.
  2. Bila `xrem ≤` kelas, kurangi dan berhenti; selain itu kosongkan kelas dan kurangi `xrem`.
  3. Akhirnya semua kelas dijaga ≥ 0.

#### 11.5.3  `filter`

- **Lokasi:** `hru/bmp.rs`, `pub fn filter(f: &Filter, area_ha: f32, surfq: f32, k1: f32, s: &mut Surface)`.
- **Padanan SWAT+:** `smp_filter` (`smp_filter.f90`), model jalur filter vegetatif White dan Arnold (2009).
- **Tujuan:** Mengurangi sedimen, N organik, nitrat, P partikulat dan P terlarut oleh jalur filter yang dibagi dua bagian: 90 % jalur menerima aliran tersebar, 10 % menerima aliran terkonsentrasi.
- **Masukan dan keluaran:** luas HRU (ha), limpasan harian `surfq` (mm), konduktivitas jenuh lapisan atas `k1` (mm/jam), konstituen `Surface`.
- **Cara kerja:**
  1. Lewati bila `surfq ≤ 0,0001` mm.
  2. Luas tangkapan bagian 1 dan 2: `D1 = (1 − con)A`, `D2 = (1 − ch)·con·A`; luas jalur `F1 = 0,9A/ratio`, `F2 = 0,1A/ratio`. Kedalaman limpasan pada jalur `q_i = (D_i/F_i)·surfq` (mm); beban sedimen `sed_i = (sed/A·1000·D_i)/(F_i·10^4)` (kg/m2).
  3. Persen pengurangan (dibatasi \[0, 100\] oleh `pct`):

```latex
\begin{aligned}
R_{q,i} &= 75{,}8 - 10{,}8\ln q_i + 25{,}9\ln K_1\\
R_{sed,i} &= 79{,}0 - 1{,}04\,sed_i + 0{,}213\,R_{q,i}\\
R_{orgN,i} &= 0{,}036\,R_{sed,i}^{1{,}69},\quad R_{NO_3,i} = 39{,}4 + 0{,}584\,R_{q,i}\\
R_{PP,i} &= 0{,}903\,R_{sed,i},\quad R_{SP,1} = 29{,}3 + 0{,}51\,R_{q,1}
\end{aligned}
```

4. Rata-rata tertimbang `R = (R_1 D1 + R_2 D2)/A` untuk tiap konstituen.
5. Sedimen: `sed = max(sed(1 − R_sed/100), 0)`, lalu `sedtrap = sed · R_sed/100` diambil dari kelas (`trap_classes`).
6. N organik, nitrat, P mineral aktif, stabil dan P organik (partikulat), lalu P terlarut dikalikan `1 − R/100` masing-masing.

- **Kebiasaan SWAT+ / catatan port:**
  - **Sedimen terjebak diambil dari kelas setelah hasil sedimen dikurangi** (memakai sedimen baru), sehingga kelas kehilangan lebih sedikit dari total.
  - **P terlarut memakai pangsa kedua P partikulat**: baris Fortran yang dimaksudkan untuk bagian 2 P terlarut menulis ke `remove21`, sehingga `remove2` masih milik P partikulat (`R_PP,2`).
  - Tidak ada koreksi `--fix-swat` untuk keduanya.

#### 11.5.4  `grass_wway`

- **Lokasi:** `hru/bmp.rs`, `pub fn grass_wway(g: &GrassWw, area_ha: f32, km: f32, surfq: f32, s: &mut Surface)`.
- **Padanan SWAT+:** `smp_grass_wway` (`smp_grass_wway.f90`).
- **Tujuan:** Menghitung pengendapan sedimen dan pengurangan hara di saluran berumput (bagian saluran dan jalur filter di sisinya).
- **Masukan dan keluaran:** parameter saluran, luas HRU (ha, km2), limpasan harian (mm), `Surface`.
- **Cara kerja:**
  1. Lewati bila `surfq ≤ 0,001` mm.
  2. Volume harian `V = 1000 · surfq · km` (m3), `V/10800`, dan debit puncak `q_p = 2 (V/10800)/(1,5 t_c)` (m3/s).
  3. Bila `q_p > vel_bf`, saluran penuh (luas dan kedalaman penuh). Selain itu naikkan kedalaman 0,01 m per iterasi, `a = (bw + 8h)h`, `p = bw + 2h√65`, hingga `qman(a, a/p, n, s) ≥ q_p`.
  4. Jalur filter di sisi saluran: `sf_area = min((d − h)·8,06·L·1000, 0,1·A·10^4)`; bila > 10^{-6}, dikalikan 0,20; `sf_depth = surfq·A·10^4/sf_area`, `sf_sed = sed·1000/sf_area`.
  5. Bila `sf_area > 0`: `R_q = clamp(95,6 − 10,79 ln sf_depth, 0, 95)`, `R_sed = clamp(79 − 1,04 sf_sed + 0,213 R_q, 0, 95)`.
  6. Sedimen masuk saluran `sedint = sed(1 − R_sed/100)`. Kecepatan `v_c = q_p/a` dibatasi `celerity_bf` (bawaan 0,001 bila `a ≤ 10^{-4}`).
  7. Bila `V/10800 > 10^{-4}`: konsentrasi masuk `c_in = sedint/V`, kapasitas angkut `c_ch = spcon · v_c^{1,5}`, pengendapan `dep = clamp(V(c_in − c_ch), 0, sedint)`.
  8. `sedout = sedint − dep`; sedimen dijaga ≥ 0,0001; `sed_frac = sedout/sed` dengan batas aneh: > 75 → 75, < 0 → 5, lalu > 30 → 30; `surq_frac = 1 − R_q/100`.
  9. `sedtrap = sed(1 − sed_frac)`; sedimen, P mineral aktif dan stabil, P organik dan N organik dikalikan `sed_frac`; P terlarut dan nitrat dikalikan `surq_frac`; kelas dikurangi `sedtrap`.
- **Kebiasaan SWAT+ / catatan port:** batas `sed_frac` (75, 5, 30) dan urutannya disalin apa adanya; secara fisika `sed_frac` seharusnya ≤ 1, tetapi batas atas tidak membatasinya ke 1. Pemaksaan sedimen ≥ 0,0001 t mengubah sedimen yang sangat kecil sebelum dikalikan.

#### 11.5.5  `user`

- **Lokasi:** `hru/bmp.rs`, `pub fn user(u: &BmpUser, s: &mut Surface, latno3: &mut f32)`.
- **Padanan SWAT+:** `smp_bmpfixed` (`smp_bmpfixed.f90`).
- **Tujuan:** Menerapkan efisiensi pembuangan tetap dari pengguna.
- **Masukan dan keluaran:** efisiensi (fraksi), `Surface`, nitrat lateral (kg/ha).
- **Cara kerja:** sedimen × `(1 − sed)`; P mineral aktif, stabil, P organik × `(1 − pp)`; P terlarut × `(1 − sp)`; N organik × `(1 − pn)`; nitrat permukaan dan nitrat lateral × `(1 − sn)`.
- **Kebiasaan SWAT+ / catatan port:** **efisiensi berupa fraksi 0–1, bukan persen seperti SWAT2012**, dan pangsa N terlarut juga mengurangi nitrat lateral. Kelas sedimen tidak dikurangi.

Validasi: Ps\_bmp (Ps\_coast dengan jalur filter riparian di kelapa sawit, saluran berumput `grwway_low` di tanaman baris, `bmpusr1` di lahan pertanian umum) cocok di semua tabel SWAT+.

### 11.6  Sistem septik dan biozona (`hru/septic.rs`)

HRU memiliki sistem septik bila tata gunanya menunjuk baris `septic.str` dengan `operation` 1 (aktif) atau 2 (gagal). Saat inisialisasi tanah (`soils_init`) sebuah lapisan biozona setebal `thk_bioz` pada kedalaman `dp_bioz` disisipkan ke profil dengan membelah lapisan; setiap hari `sep_biozone` mengubah kapasitas lapang dan kejenuhan lapisan itu menurut biomassa dan plak, serta mengubah dan memindahkan amonium, nitrat dan P terlarut (model Jeong dkk. 2011). Ciri penting port: **SWAT+ tidak pernah mengisi efluen tangki septik** (`qstemm` hanya dinolkan), sehingga tidak ada air maupun muatan yang masuk biozona; port mengikutinya bit demi bit (`docs/structures.md`).

**Struct `Septic`**

| Field | Arti | Satuan |
| --- | --- | --- |
| `isep` | Baris `septic.str` (`iseptic`) | – |
| `yr`, `tfail` | Tahun mulai beroperasi; lama kegagalan sebelum pulih | tahun, hari |
| `r: [f32; 23]` | Parameter baris `septic.str` (diakses dengan konstanta `S::Z`, `S::THK`, `S::BD`, `S::FC1`, `S::FC2`, `S::RSP`, `S::MRT`, `S::SLG1`, `S::SLG2`, `S::PLQ`, `S::BOD_DC`, `S::BOD_CONV`, `S::FECAL`, `S::NITR`, `S::DENITR`, `S::PDISTRB`, `S::PSORPMAX`, `S::SOLPSLP`, `S::SOLPINTC`, dll.) | campuran (mm, g/cm3, 1/hari, …) |
| `eff: [f32; 10]` | Mutu efluen `septic.sep`: q\_rate, BOD, TSS, NH4, NO3, NO2, orgN, minP, orgP, fcoli | mg/L, cfu/100 mL |
| `bz` | Indeks lapisan biozona (0-based, `i_sep − 1`) | – |
| `biom`, `plqm` | Biomassa bakteri dan plak | kg/ha |
| `bio_bod`, `fcoli` | BOD dalam biozona, koliform tinja | kg/ha, cfu/100 mL |
| `rbiom`, `bz_perc` | Laju perubahan biomassa harian; perkolasi dari biozona | kg/ha, mm |
| `tsincefail` | Hari sejak gagal | hari |

#### 11.6.1  `layersplit`

- **Lokasi:** `hru/septic.rs`, `pub fn layersplit(layers: &mut Vec<Layer>, dep_new: f32) -> Result<(), String>`.
- **Padanan SWAT+:** `layersplit` (`layersplit.f90`).
- **Tujuan:** Membelah lapisan pertama di bawah lapisan atas yang dasarnya lebih dalam dari `dep_new` (mm) menjadi dua.
- **Masukan dan keluaran:** profil lapisan, kedalaman belah (mm); profil bertambah satu lapisan, atau galat bila tidak ada lapisan seperti itu.
- **Cara kerja:**
  1. Telusuri lapisan mulai indeks 1 (lapisan kedua).
  2. Pada lapisan pertama dengan `d > dep_new`: salin lapisan; bagian bawah mendapat `thick = d_asli − dep_new`; bagian atas `d = dep_new`, `thick = dep_new − d_lapisan_atasnya`; sisipkan bagian bawah setelahnya.
  3. Tanpa lapisan yang cocok kembalikan galat (SWAT+ akan tertinggal tanpa lapisan).
- **Kebiasaan SWAT+ / catatan port:** **kedua bagian mempertahankan air dan kapasitas lapisan asli dalam mm** (hanya kedalaman dan tebal yang berubah), sehingga profil yang dibelah menampung lebih banyak air.

#### 11.6.2  `biozone`

- **Lokasi:** `hru/septic.rs`, `pub fn biozone(layers: &mut Vec<Layer>, sep: &S) -> Result<usize, String>`.
- **Padanan SWAT+:** blok septik di `soils_init` (`soils_init.f90`).
- **Tujuan:** Menyisipkan lapisan biozona bagi sistem yang beroperasi dan menentukan indeksnya (`i_sep`).
- **Masukan dan keluaran:** profil, parameter septik (kedalaman `z` dan tebal `thk` biozona, mm); indeks biozona 0-based atau galat.
- **Cara kerja:**
  1. Bila `z + thk` melampaui dasar profil: `i_sep = nly + 1`, belah di `z − thk`.
  2. Selain itu, cari lapisan pertama (mulai indeks 1) dengan `z < d`: `i_sep = ly + 2` (penomoran Fortran berbasis 1 setelah penyisipan). Bila `|z − d| > 25,4` mm, belah di `z` dan `z + thk`; selain itu dasar lapisan dipindah ke `z`. Bila `|z − d_{ly−1}| < 25,4` mm (biozona mulai dalam satu inci dari puncak lapisan), cukup belah di `z + thk`.
  3. Lakukan `layersplit` untuk tiap kedalaman belah > 10^{-6}.
  4. Tolak bila `i_sep < 3` atau `i_sep ≥` jumlah lapisan (SWAT+ akan membaca di luar tanah atau lapisan 0; biozona wajib punya lapisan di bawahnya). Kembalikan `i_sep − 1`.
- **Kebiasaan SWAT+ / catatan port:** **bila biozona mulai dalam satu inci dari puncak lapisan, `i_sep` adalah lapisan di bawahnya**, dan lapisan itulah yang diperlakukan sebagai biozona. Sistem septik dengan model karbon (`cswat` 1 atau 2) ditolak di tempat lain.

#### 11.6.3  `sep_biozone`

- **Lokasi:** `hru/septic.rs`, `pub fn sep_biozone(sp: &mut Septic, layers: &mut [Layer], area_ha: f32, opt: &mut i32)`.
- **Padanan SWAT+:** `sep_biozone` (`sep_biozone.f90`).
- **Tujuan:** Proses harian biozona: kegagalan dan pemulihan, penyumbatan (kapasitas lapang naik, kejenuhan turun oleh plak), pertumbuhan biomassa, peluruhan BOD dan koliform, nitrifikasi, denitrifikasi, sorpsi P, dan perkolasi NH4 dan P ke lapisan bawah.
- **Masukan dan keluaran:** state `Septic`, profil, luas HRU (ha), penanda operasi bersama tipe septik `opt` (1 aktif, 2 gagal). Memperbarui `fc`, `ul`, `no3`, `nh4`, `lab` (P labil/terlarut), `act`, `hsta` lapisan biozona dan lapisan di bawahnya.
- **Cara kerja:**
  1. `qstemm = 0`. Hitung dalam f64: `bza = A`, `bz_vol = thk · A · 10`, `qi = (st_bz + prk_{bz−1} + qstemm) · A · 10`, `qin = qstemm · A · 10` (= 0), `qout = bz_perc · A · 10`; faktor `rtof = 0,5`, `ctmp = 1`.
  2. **Bila gagal (`opt = 2`)**: tambah `tsincefail` hanya bila sudah > 0; bila `tsincefail ≥ tfail`, pulihkan (`opt = 1`, `ul = thk(φ − wp)`, `fc = thk(up − wp)`, hara dan state dinolkan). Selesai.
  3. Tambahkan muatan efluen `xx = qin/A/1000` (selalu 0): NO3 + NO2 ke no3, NH4, orgN dan orgP × `rtof` ke humus, minP ke P labil; BOD ke `bio_bod`. `bodi = bio_bod · A/qi · 1000`.
  4. Penyumbatan:

```latex
FC \mathrel{+}= \frac{fc_1\,(UL - FC)^{fc_2}\;rbiom}{10\,\rho_b},\qquad UL = \phi\,thk - \frac{plqm}{10\,\rho_b}
```

Bila `UL ≤ FC`, maka `UL = FC` dan sistem **gagal** (`opt = 2`).

5. Laju bakteri: respirasi `rrsp = rsp · biom`, kematian `rmort = mrt · biom`, pelepasan `rslg = slg1 · bz_perc^{slg2} · biom`; plak `rplqm = max(rmort − rslg + plq · qin · TSS/(1000 A), 0)`, `plqm += rplqm`.
6. Koliform bercampur: `xx = 10 st A/(qin + 10 st A)`, `fcoli = fcoli · xx + fcoli_eff(1 − xx)`.
7. Laju reaksi relatif `rtrate = biom · A/(bz_vol · φ)`. BOD: `k = clamp(bod_dc · rtrate, 0, 4)`, `rbod = bodi(1 − e^{−k})`, `bode = bodi − rbod`, `bio_bod = bode · 10 st/1000`. Koliform: `fcoli −= fcoli(1 − e^{−fecal·rtrate})`.
8. Nitrifikasi: `rnit = NH4(1 − e^{−nitr·rtrate})`, pindahkan ke NO3. Perkolasi NH4: `nperc = min(0,2 · qout/qi · NH4, 0,5 NH4)`, pindah ke lapisan `bz+1`.
9. Denitrifikasi: `NO3 −= NO3(1 − e^{−denitr·rtrate})`.
10. Sorpsi P: `svolp = (d_dasar − z) · A · 10`, `solpconc = lab · A/qi · 1000`, `solpsorb = 1,6·10^{-3} · min(pdistrb · solpconc, psorpmax) · svolp (1 − φ)`. Bila `lab · A < solpsorb`: `totalp = lab + act`, `solp_end = min(solpslp · totalp + solpintc, lab)`, `act = act + lab − solp_end`, `lab = solp_end`.
11. Perkolasi P: `percp = 0,01 · solpconc · qout/A · 10^{-3}` dari `lab` ke `lab` lapisan `bz+1`.
12. Biomassa: `rbiom = max(ctmp · bod_conv (qin · BOD_eff − qout · bode)/(1000 A) − (rrsp + rmort + rslg), 10^{-6})`; `biom += rbiom`.

- **Kebiasaan SWAT+ / catatan port:**
  - **Presisi campuran**: variabel lokal Fortran `real*8` (bza, qi, qin, qout, xx, rtrate, laju, nperc, solpconc, …) dihitung dalam f64 di port, sedangkan field tanah dan state tetap f32, dikonversi tepat di titik penugasan yang sama. Literal Fortran tanpa `d0` (0,2; 1,6e-3; 0,01; 1,e-6) adalah real tunggal, sehingga port menulisnya `0.2_f32 as f64` dsb. agar nilai biner sama.
  - **Tidak ada efluen**: `qstemm` hanya dinolkan; BOD, N, P dan bakteri efluen tak pernah masuk, dan `qi` hanya dari air tanah.
  - **Biozona gagal pada hari pertamanya lalu menyegel tanah**: kejenuhan direset ke `φ · thk_bioz` sementara kapasitas lapang tetap milik seluruh lapisan asal, sehingga biasanya `UL ≤ FC` seketika. Waktu perkolasi biozona gagal 10^{10} jam (di rutin perkolasi). Tidak pernah pulih karena `tsincefail` hanya bertambah bila sudah > 0 dan tidak ada yang memulainya.
  - **Operasi milik tipe septik, bukan HRU**: `opt` adalah `sep(isep)%opt`; kegagalan satu HRU menggagalkan semua HRU dengan baris `septic.str` yang sama, mulai HRU berikutnya yang dihitung hari itu.
  - Di luar fungsi ini (dari `docs/structures.md`): tanah dari biozona ke bawah dijaga lebih hangat setelah tahun pertama sistem; biozona aktif dikecualikan dari nitrifikasi `nut_nitvol` dan denitrifikasi `nut_nminrl`; setiap lapisan biozona dikecualikan dari pencucian P terlarut.
  - Validasi Ps\_septic (sistem `standard` di 12 HRU lahan kering): semua tabel cocok; pada HRU itu perkolasi turun hampir nol dan limpasan naik tajam.

### 11.7  Rekap fungsi dan skenario validasi

Bab ini mendokumentasikan 25 fungsi: 7 di `wetland.rs`, 5 di `subdaily.rs`, 8 di `bmp.rs`, 3 di `septic.rs`, dan 2 method urban di `mod.rs`.

| Fungsi | File | Padanan SWAT+ | Entri |
| --- | --- | --- | --- |
| `find` | wetland.rs | – (khusus port) | 11.1.1 |
| `Wetland::new` | wetland.rs | `wet_initial` | 11.1.2 |
| `Wetland::set_weir` | wetland.rs | aksi `weir_height` | 11.1.3 |
| `Wetland::weir_release` | wetland.rs | `res_weir_release` | 11.2.1 |
| `Wetland::release` | wetland.rs | `conditions`, `res_hydro` | 11.2.2 |
| `Hru::wetland_control` | wetland.rs | `wetland_control` | 11.2.3 |
| `Hru::wet_excess` | wetland.rs | `swr_satexcess` | 11.2.4 |
| `wfsh` | subdaily.rs | `soils_init` | 11.3.1 |
| `Hru::green_ampt` | subdaily.rs | `sq_greenampt` | 11.3.2 |
| `Hru::ero_ovrsed` | subdaily.rs | `ero_ovrsed` | 11.3.3 |
| `Hru::lag_sediment` | subdaily.rs | `stor_surfstor` | 11.3.4 |
| `Hru::lag_runoff` | subdaily.rs | `sq_surfst` | 11.3.5 |
| `Hru::hru_urban` | mod.rs | `hru_urban`, `regres` | 11.4.1 |
| `Hru::hru_urbanhr` | mod.rs | `hru_urbanhr` | 11.4.2 |
| `qman`, `find`, `pct` | bmp.rs | `Qman`; –; batas di `smp_filter` | 11.5 (tabel) |
| `init` | bmp.rs | `structure_init`, `structure_set_parms`, `ttcoef_wway` | 11.5.1 |
| `trap_classes` | bmp.rs | blok kelas di `smp_*` | 11.5.2 |
| `filter` | bmp.rs | `smp_filter` | 11.5.3 |
| `grass_wway` | bmp.rs | `smp_grass_wway` | 11.5.4 |
| `user` | bmp.rs | `smp_bmpfixed` | 11.5.5 |
| `layersplit` | septic.rs | `layersplit` | 11.6.1 |
| `biozone` | septic.rs | `soils_init` | 11.6.2 |
| `sep_biozone` | septic.rs | `sep_biozone` | 11.6.3 |

Skenario validasi yang menguji modul bab ini (semua cocok bit demi bit dengan SWAT+ pada tabel yang dicetak, menurut `docs/`):

| Skenario | Modul yang diuji |
| --- | --- |
| Osu\_1hru | Sawah SWAT+ sendiri: ambang acak (`prob_unif`), irigasi genangan, pelumpuran, tanam/panen padi oleh tabel keputusan |
| Ps\_paddy | HRU padi Ps\_coast sebagai sawah, dua musim setahun |
| Ps\_hourly | Langkah per jam dengan Green dan Ampt (hujan harian disebar 4 jam menurut distribusi Mononobe) |
| Ps\_urbhr | Ps\_hourly dengan HRU kota kepadatan sedang (`urmd`), penumpukan/pencucian per langkah |
| Ps\_gap, Ps\_uh | Celah hujan subharian dan stasiun hujan dibangkitkan; bilangan kurva dengan hidrograf satuan gamma |
| Ps\_bmp | Jalur filter, saluran berumput, BMP pengguna |
| Ps\_septic | Sistem septik `standard` di 12 HRU lahan kering |

Koreksi `--fix-swat` yang menyentuh bab ini: koefisien evaporasi lahan basah milik sendiri (`et_act`), pupuk ke air sawah tergenang dengan model karbon apa pun (`pl_fert_wet`), dan hara genangan ikut tercampur saat pelumpuran (`mgt_newtillmix_wet`). Dua yang terakhir berada di modul manajemen, bukan di file bab ini.
