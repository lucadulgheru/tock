//! USB on-the-go full-speed (OTG_FS)
// NOTE: This is not a full implementation of OTG. Only device mode is supported at the moment.

use core::cell::Cell;
use kernel::common::cells::{OptionalCell, VolatileCell};
use kernel::common::registers::interfaces::{ReadWriteable, Readable, Writeable};
use kernel::common::registers::{register_bitfields, register_structs, ReadWrite};
use kernel::common::StaticRef;
use kernel::hil;
use kernel::hil::usb::TransferType;

use kernel::debug;
use kernel::ClockInterface;
use stm32f4xx::rcc;

macro_rules! internal_err {
    [ $( $arg:expr ),+ ] => {
        panic!($( $arg ),+);
    };
}

register_structs! {
        pub UsbotgRegisters {
            // Global CSR map
            (0x000 => gotgctl: ReadWrite<u32, GOTGCTL::Register>),
            (0x004 => gotgint: ReadWrite<u32, GOTGINT::Register>),
            (0x008 => gahbcfg: ReadWrite<u32, GAHBCFG::Register>),
            (0x00C => gusbcfg: ReadWrite<u32, GUSBCFG::Register>),
            (0x010 => grstctl: ReadWrite<u32, GRSTCTL::Register>),
            (0x014 => gintsts: ReadWrite<u32, GINTSTS::Register>),
            (0x018 => gintmsk: ReadWrite<u32, GINTMSK::Register>),
            (0x01C => grxstsr: ReadWrite<u32, GRXSTSR::Register>),
            (0x020 => grxstsp: ReadWrite<u32, GRXSTSP::Register>),
            (0x024 => grxfsiz: ReadWrite<u32, GRXFSIZ::Register>),
            (0x028 => dieptxf0: ReadWrite<u32, DIEPTXF0::Register>),
            (0x02C => hnptxsts: ReadWrite<u32, HNPTXSTS::Register>),
            (0x030 => _reserved0),
            (0x038 => gccfg: ReadWrite<u32, GCCFG::Register>),
            (0x03C => cid: ReadWrite<u32, CID::Register>),
            (0x040 => _reservedx),
            (0x054 => glpmcfg: ReadWrite<u32, GLPMCFG::Register>),
            (0x058 => _reserved1),
            (0x100 => hptxfsiz: ReadWrite<u32, HPTXFSIZ::Register>),
            (0x104 => dieptxfx: [ReadWrite<u32, DIEPTXFx::Register>; 5]),
            (0x118 => _reserved2),
            // Host-mode CSR map
            (0x400 => hcfg: ReadWrite<u32, HCFG::Register>),
            (0x404 => hfir: ReadWrite<u32, HFIR::Register>),
            (0x408 => hfnum: ReadWrite<u32, HFNUM::Register>),
            (0x40C => _reserved3),
            (0x410 => hptxsts: ReadWrite<u32, HPTXSTS::Register>),
            (0x414 => haint: ReadWrite<u32, HAINT::Register>),
            (0x418 => haintmsk: ReadWrite<u32, HAINTMSK::Register>),
            (0x41C => _reserved4),
            (0x440 => hprt: ReadWrite<u32, HPRT::Register>),
            (0x444 => _reserved5),
            (0x500 => host_channel_registers: [HostChannelRegisters; 12]),
            (0x680 => _reserved29),
            // Device-mode CSR map
            (0x800 => dcfg: ReadWrite<u32, DCFG::Register>),
            (0x804 => dctl: ReadWrite<u32, DCTL::Register>),
            (0x808 => dsts: ReadWrite<u32, DSTS::Register>),
            (0x80C => _reserved30),
            (0x810 => diepmsk: ReadWrite<u32, DIEPMSK::Register>),
            (0x814 => doepmsk: ReadWrite<u32, DOEPMSK::Register>),
            (0x818 => daint: ReadWrite<u32, DAINT::Register>),
            (0x81C => daintmsk: ReadWrite<u32, DAINTMSK::Register>),
            (0x820 => _reserved31),
            (0x828 => dvbusdis: ReadWrite<u32, DVBUSDIS::Register>),
            (0x82C => dvbuspulse: ReadWrite<u32, DVBUSPULSE::Register>),
            (0x830 => _reserved32),
            (0x834 => diepempmsk: ReadWrite<u32, DIEPEMPMSK::Register>),
            (0x838 => _reserved33),
            (0x900 => endpoint0_in_registers: Endpoint0InRegisters),
            //(0x91C => _reserved37),
            (0x920 => endpoint_in_registers: [EndpointInRegisters; 5]),
            (0x9C0 => _reserved_spc),
            (0xB00 => endpoint0_out_registers: Endpoint0OutRegisters),
            //(0xB14 => _reserved60),
            (0xB20 => endpoint_out_registers: [EndpointOutRegisters; 5]),
            (0xBC0 => _reserved_ceva),
            (0xE00 => pcgcctl: ReadWrite<u32, PCGCCTL::Register>),
            (0xE04 => @END),
        },

        pub HostChannelRegisters{
            (0x000 => hccharx: ReadWrite<u32, HCCHARx::Register>),
            (0x004 => _reserved_hc),
            (0x008 => hcintx: ReadWrite<u32, HCINTx::Register>),
            (0x00C => hcintmskx: ReadWrite<u32, HCINTMSKx::Register>),
            (0x010 => hctsizx: ReadWrite<u32, HCTSIZx::Register>),
            (0x014 => _reserved_hc2),
            (0x020 => @END),
        },

        pub Endpoint0InRegisters{
            (0x000 => diepctl0: ReadWrite<u32, DIEPCTL0::Register>),
            (0x004 => _reserved34),
            (0x008 => diepint0: ReadWrite<u32, DIEPINTx::Register>),
            (0x00C => _reserved35),
            (0x010 => dieptsiz0: ReadWrite<u32, DIEPTSIZ0::Register>),
            (0x014 => _reserved36),
            (0x018 => dtxfsts0: ReadWrite<u32, DTXFSTSx::Register>),
            (0x01C => _reserved37),
            (0x020 => @END),
        },

        pub EndpointInRegisters{
            (0x000 => diepctlx: ReadWrite<u32, DIEPCTLx::Register>),
            (0x004 => _reserved_0),
            (0x008 => diepintx: ReadWrite<u32, DIEPINTx::Register>),
            (0x00C => _reserved_1),
            (0x010 => dieptsizx: ReadWrite<u32, DIEPTSIZx::Register>),
            (0x014 => _reserved_2),
            (0x018 => dtxfstsx: ReadWrite<u32, DTXFSTSx::Register>),
            (0x01C => _reserved_3),
            (0x020 => @END),
        },

        pub Endpoint0OutRegisters{
            (0x000 => doepctl0: ReadWrite<u32, DOEPCTL0::Register>),
            (0x004 => _reserved58),
            (0x008 => doepint0: ReadWrite<u32, DOEPINTx::Register>),
            (0x00C => _reserved59),
            (0x010 => doeptsiz0: ReadWrite<u32, DOEPTSIZ0::Register>),
            (0x014 => _reserved60),
            (0x020 => @END),
        },

        pub EndpointOutRegisters{
            (0x000 => doepctlx: ReadWrite<u32, DOEPCTLx::Register>),
            (0x004 => _reserved_4),
            (0x008 => doepintx: ReadWrite<u32, DOEPINTx::Register>),
            (0x00C => _reserved_5),
            (0x010 => doeptsizx: ReadWrite<u32, DOEPTSIZx::Register>),
            (0x014 => _reserved_6),
            (0x020 => @END),
        }
    }
    
register_bitfields![u32, 

    GOTGCTL [
           // Current mode of operation
           CURMOD OFFSET(21) NUMBITS(1) [],
           // OTG version
           OTGVER OFFSET(20) NUMBITS(1) [],
           // B-session valid
           BSVLD OFFSET(19) NUMBITS(1) [],
           // A-session valid
           ASVLD OFFSET(18) NUMBITS(1) [],
           // Long/short debounce time
           DBCT OFFSET(17) NUMBITS(1) [],
           // Connector ID status
           CIDSTS OFFSET(16) NUMBITS(1) [],
           // Embedded host enable
           EHEN OFFSET(12) NUMBITS(1) [],
           // Device HNP enabled
           DHNPEN OFFSET(11) NUMBITS(1) [],
           // Host set HNP enable
           HSHNPEN OFFSET(10) NUMBITS(1) [],
           // HNP request
           HNPRQ OFFSET(9) NUMBITS(1) [],
           // Host negotiation success
           HNGSCS OFFSET(8) NUMBITS(1) [],
           // B-peripheral session valid override value
           BVALOVAL OFFSET(7) NUMBITS(1) [],
           // B-peripheral session valid override enable
           BVALOEN OFFSET(6) NUMBITS(1) [],
           // A-peripheral session valid override value
           AVALOVAL OFFSET(5) NUMBITS(1) [],
           // A-peripheral session valid override enable
           AVALOEN OFFSET(4) NUMBITS(1) [],
           // V BUS valid override value
           VBVALOVAL OFFSET(3) NUMBITS(1) [],
           // V BUS valid override enable
           VBVALOEN OFFSET(2) NUMBITS(1) [],
           // Session request
           SRQ OFFSET(1) NUMBITS(1) [],
           // Session request success
           SRQSCS OFFSET(0) NUMBITS(1) []
    ],

    GOTGINT [
            // Debounce done
            DBCDNE OFFSET(19) NUMBITS(1) [], 
            // A-device timeout change
            ADTOCHG OFFSET(18) NUMBITS(1) [],
            // Host negotiation detected
            HNGDET OFFSET(17) NUMBITS(1) [],
            // Host negotiation success status change
            HNSSCHG OFFSET(9) NUMBITS(1) [],
            // Session request success status change
            SRSSCHG OFFSET(8) NUMBITS(1) [],
            // Session end detected
            SEDET OFFSET(2) NUMBITS(1) []
    ],

    GAHBCFG [
            // Periodic Tx FIFO empty level
            PTXFELVL OFFSET(8) NUMBITS(1) [],
            // Tx FIFO empty level
            TXFELVL OFFSET(7) NUMBITS(1) [],
            // Global interrupt mask
            GINTMSK OFFSET(0) NUMBITS(1) []
    ],

    GUSBCFG [
            // Force device mode
            FDMOD OFFSET(30) NUMBITS(1) [],
            // Force host mode
            FHMOD OFFSET(29) NUMBITS(1) [],
            // USB turnaround time
            TRDT OFFSET(10) NUMBITS(4) [],
            // HNP-capable
            HNPCAP OFFSET(9) NUMBITS(1) [],
            // SRP-capable
            SRPCAP OFFSET(8) NUMBITS(1) [],
            // Full Speed serial transceiver mode select
            PHYSEL OFFSET(6) NUMBITS(1) [],
            // FS timeout calibration
            TOCAL OFFSET(0) NUMBITS(3) []
    ],

    GRSTCTL [
            // AHB master idle
            AHBIDL OFFSET(31) NUMBITS(1) [],
            // Tx FIFO number
            TXFNUM OFFSET(6) NUMBITS(5) [],
            // Tx FIFO flush
            TXFFLSH OFFSET(5) NUMBITS(1) [],
            // Rx FIFO flush
            RXFFLSH OFFSET(4) NUMBITS(1) [],
            // Host frame counter reset
            FCRST OFFSET(2) NUMBITS(1) [],
            // Partial soft reset
            PSRST OFFSET(1) NUMBITS(1) [],
            // Core soft reset
            CSRST OFFSET(0) NUMBITS(1) []
    ],

    GINTSTS [
            // Resume/remote wakeup detected interrupt
            WKUPINT OFFSET(31) NUMBITS(1) [],
            // Session request/new session detected interrupt
            SRQINT OFFSET(30) NUMBITS(1) [],
            // Disconnect detected interrupt
            DISCINT OFFSET(29) NUMBITS(1) [],
            // Connector ID status change
            CIDSCHG OFFSET(28) NUMBITS(1) [],
            // LPM interrupt
            LPMINT OFFSET(27) NUMBITS(1) [],
            // Periodic Tx FIFO empty
            PTXFE OFFSET(26) NUMBITS(1) [],
            // Host channels interrupt
            HCINT OFFSET(25) NUMBITS(1) [],
            // Host port interrupt
            HPRTINT OFFSET(24) NUMBITS(1) [],
            // Reset detected interrupt
            RSTDET OFFSET(23) NUMBITS(1) [],
            // Incomplete periodic transfer
            IPXFR OFFSET(21) NUMBITS(1) [],
            // Incomplete isochronous OUT transfer
            INCOMPISOOUT OFFSET(21) NUMBITS(1) [],
            // Incomplete isochronous IN transfer
            IISOIXFR OFFSET(20) NUMBITS(1) [],
            // OUT endpoint interrupt
            OEPINT OFFSET(19) NUMBITS(1) [],
            // IN endpoint interrupt
            IEPINT OFFSET(18) NUMBITS(1) [],
            // End of periodic frame interrupt
            EOPF OFFSET(15) NUMBITS(1) [],
            // Isochronous OUT packet dropped interrupt
            ISOODRP OFFSET(14) NUMBITS(1) [],
            // Enumeration done
            ENUMDNE OFFSET(13) NUMBITS(1) [],
            // USB reset
            USBRST OFFSET(12) NUMBITS(1) [],
            // USB suspend
            USBSUSP OFFSET(11) NUMBITS(1) [],
            // Early suspend
            ESUSP OFFSET(10) NUMBITS(1) [],
            // Global OUT NAK effective
            GONAKEFF OFFSET(7) NUMBITS(1) [],
            // Global IN non-periodic NAK effective
            GINAKEFF OFFSET(6) NUMBITS(1) [],
            // Non-periodic Tx FIFO empty
            NPTXFE OFFSET(5) NUMBITS(1) [],
            // Rx FIFO non-empty
            RXFLVL OFFSET(4) NUMBITS(1) [],
            // Start of frame
            SOF OFFSET(3) NUMBITS(1) [],
            // OTG interrupt
            OTGINT OFFSET(2) NUMBITS(1) [],
            // Mode mismatch interrupt
            MMIS OFFSET(1) NUMBITS(1) [],
            // Current mode of operation
            CMOD OFFSET(0) NUMBITS(1) []
    ],

    GINTMSK [
            // Resume/remote wakeup detected interrupt mask
            WUIM OFFSET(31) NUMBITS(1) [],
            // Session request/new session detected interrupt mask
            SRQIM OFFSET(30) NUMBITS(1) [],
            // Disconnect detected interrupt mask
            DISCINT OFFSET(29) NUMBITS(1) [],
            // Connector ID status change mask
            CIDSCHGM OFFSET(28) NUMBITS(1) [],
            // LPM interrupt mask
            LPMINTM OFFSET(27) NUMBITS(1) [],
            // Periodic Tx FIFO empty mask
            PTXFEM OFFSET(26) NUMBITS(1) [],
            // Host channels interrupt mask
            HCIM OFFSET(25) NUMBITS(1) [],
            // Host port interrupt mask
            PRTIM OFFSET(24) NUMBITS(1) [],
            // Reset detected interrupt mask
            RSTDETM OFFSET(23) NUMBITS(1) [],
            // Incomplete periodic transfer mask
            IPXFRM OFFSET(21) NUMBITS(1) [],
            // Incomplete isochronous OUT transfer mask
            IISOOXFRM OFFSET(21) NUMBITS(1) [],
            // Incomplete isochronous IN transfer mask
            IISOIXFRM OFFSET(20) NUMBITS(1) [],
            // OUT endpoints interrupt mask
            OEPINT OFFSET(19) NUMBITS(1) [],
            // IN endpoints interrupt mask
            IEPINT OFFSET(18) NUMBITS(1) [],
            // End of periodic frame interrupt mask
            EOPFM OFFSET(15) NUMBITS(1) [],
            // Isochronous OUT packet dropped interrupt mask
            ISOODRPM OFFSET(14) NUMBITS(1) [],
            // Enumeration done mask
            ENUMDNEM OFFSET(13) NUMBITS(1) [],
            // USB reset mask
            USBRST OFFSET(12) NUMBITS(1) [],
            // USB suspend mask
            USBSUSPM OFFSET(11) NUMBITS(1) [],
            // Early suspend mask
            ESUSPM OFFSET(10) NUMBITS(1) [],
            // Global OUT NAK effective mask
            GONAKEFFM OFFSET(7) NUMBITS(1) [],
            // Global non-periodic IN NAK effective mask
            GINAKEFFM OFFSET(6) NUMBITS(1) [],
            // Non-periodic Tx FIFO empty mask
            NPTXFEM OFFSET(5) NUMBITS(1) [],
            // Receive FIFO non-empty mask
            RXFLVLM OFFSET(4) NUMBITS(1) [],
            // Start of frame mask
            SOFM OFFSET(3) NUMBITS(1) [],
            // OTG interrupt mask
            OTGINT OFFSET(2) NUMBITS(1) [],
            // Mode mismatch interrupt mask
            MMISM OFFSET(1) NUMBITS(1) []
    ],

    GRXSTSR [
            // Status phase start
            STSPHST OFFSET(27) NUMBITS(1) [],
            // Frame number
            FRMNUM OFFSET(21) NUMBITS(4) [],
            // Packet status
            PKTSTS OFFSET(17) NUMBITS(4) [],
            // Data PID
            DPID OFFSET(15) NUMBITS(2) [],
            // Byte count
            BCNT OFFSET(4) NUMBITS(11) [],
            // Endpoint number
            EPNUM OFFSET(0) NUMBITS(4) []
    ],

    GRXSTSP [
            // Status phase start
            STSPHST OFFSET(27) NUMBITS(1) [],
            // Frame number
            FRMNUM OFFSET(21) NUMBITS(4) [],
            // Packet status
            PKTSTS OFFSET(17) NUMBITS(4) [],
            // Data PID
            DPID OFFSET(15) NUMBITS(2) [],
            // Byte count
            BCNT OFFSET(4) NUMBITS(11) [],
            // Endpoint number
            EPNUM OFFSET(0) NUMBITS(4) []
    ],

    GRXFSIZ [
            // Rx FIFO depth
            RXFD OFFSET(0) NUMBITS(16) []
    ],

    DIEPTXF0 [
            // Endpoint 0 Tx FIFO depth
            TX0FD OFFSET(16) NUMBITS(16) [],
            // Endpoint 0 transmit RAM start address
            TX0FSA OFFSET(0) NUMBITS(16) []
    ],

    HNPTXSTS [
            // Top of the non-periodic transmit request queue
            NPTXQTOP OFFSET(24) NUMBITS(7) [],
            // Non-periodic transmit request queue space available
            NPTQXSAV OFFSET(16) NUMBITS(8) [],
            // Non-periodic Tx FIFO space available
            NPTXFSAV OFFSET(0) NUMBITS(16) []
    ],


    GCCFG [
            // USB V BUS detection enable
            VBDEN OFFSET(21) NUMBITS(1) [],
            // Secondary detection (SD) mode enable
            SDEN OFFSET(20) NUMBITS(1) [],
            // Primary detection (PD) mode enable
            PDEN OFFSET(19) NUMBITS(1) [],
            // Data contact detection (DCD) mode enable
            DCDEN OFFSET(18) NUMBITS(1) [],
            // Battery charging detector (BCD) enable
            BCDEN OFFSET(17) NUMBITS(1) [],
            // Power down control of FS PHY
            PWRDWN OFFSET(16) NUMBITS(1) [],
            // DM pull-up detection status
            PS2DET OFFSET(3) NUMBITS(1) [],
            // Secondary detection (SD) status
            SDET OFFSET(2) NUMBITS(1) [],
            // Primary detection (PD) status
            PDET OFFSET(1) NUMBITS(1) [],
            // Data contact detection (DCD) status
            DCDET OFFSET(0) NUMBITS(1) []
    ],
    
    CID [
            // Product ID field
            PRODUCT_ID OFFSET(0) NUMBITS(32) []
    ],
    
    GLPMCFG [
            // Enable best effort service latency
            ENBESL OFFSET(28) NUMBITS(1) [],
            // LPM retry count status
            LPMRCNTSTS OFFSET(25) NUMBITS(3) [],
            // Send LPM transaction
            SNDLPM OFFSET(24) NUMBITS(1) [],
            // LPM retry count
            LPMRCNT OFFSET(21) NUMBITS(3) [],
            // LPM Channel Index
            LPMCHIDX OFFSET(17) NUMBITS(4) [],
            // Sleep state resume OK
            L1RSMOK OFFSET(16) NUMBITS(1) [],
            // Port sleep status
            SLPSTS OFFSET(15) NUMBITS(1) [],
            // LPM response
            LPMRSP OFFSET(13) NUMBITS(2) [],
            // L1 deep sleep enable
            L1DSEN OFFSET(12) NUMBITS(1) [],
            // BESL threshold
            BESLTHRS OFFSET(8) NUMBITS(4) [],
            // L1 Shallow Sleep enable
            L1SSEN OFFSET(7) NUMBITS(1) [],
            // RemoteWake value
            REMWAKE OFFSET(6) NUMBITS(1) [],
            // Best effort service latency
            BESL OFFSET(2) NUMBITS(4) [],
            // LPM token acknowledge enable
            LPMACK OFFSET(1) NUMBITS(1) [],
            // LPM support enable
            LPMEN OFFSET(0) NUMBITS(1) []
    ],

    HPTXFSIZ [
            // Host periodic Tx FIFO depth
            PTXFSIZ OFFSET(16) NUMBITS(16) [],
            // Host periodic Tx FIFO start address
            PTXSA OFFSET(0) NUMBITS(16) []
    ],
    

    DIEPTXFx [
            // IN endpoint Tx FIFO depth
            INEPTXFD OFFSET(16) NUMBITS(16) [],
            // IN endpoint FIFOx transmit RAM start address
            INEPTXSA OFFSET(0) NUMBITS(16) []
    ],    
    // Host-mode registers


    HCFG [
            // FS- and LS-only support
            FSLSS OFFSET(2) NUMBITS(1) [],
            // FS/LS PHY clock select
            FSLSPCS OFFSET(0) NUMBITS(2) []
    ],


    HFIR [
            // Reload control
            RLDCTRL OFFSET(16) NUMBITS(1) [],
            // Frame interval
            FRIVL OFFSET(0) NUMBITS(16) []
    ],


    HFNUM [
            // Frame time remaining
            FTREM OFFSET(16) NUMBITS(16) [],
            // Frame number
            FRNUM OFFSET(0) NUMBITS(16) []
    ],
    
    HPTXSTS [
            // Top of the periodic transmit request queue
            PTXQTOP OFFSET(24) NUMBITS(8) [],
            // Periodic transmit request queue space available
            PTXQSAV OFFSET(16) NUMBITS(8) [],
            // Periodic transmit data FIFO space available
            PTXFSAVL OFFSET(0) NUMBITS(16) []
    ],

    HAINT [
            // Channel interrupts
            HAINT OFFSET(0) NUMBITS(16) []
    ],
    

    HAINTMSK [
            // Channel interrupt mask
            HAINTM OFFSET(0) NUMBITS(16) []
    ],

    HPRT [
            // Port speed
            PSPD OFFSET(17) NUMBITS(2) [],
            // Port test control
            PTCTL OFFSET(13) NUMBITS(4) [],
            // Port power
            PPWR OFFSET(12) NUMBITS(1) [],
            // Port line status
            PLSTS OFFSET(10) NUMBITS(2) [],
            // Port reset
            PRST OFFSET(8) NUMBITS(1) [],
            // Port suspend
            PSUSP OFFSET(7) NUMBITS(1) [],
            // Port resume
            PRES OFFSET(6) NUMBITS(1) [],
            // Port overcurrent change
            POCCHNG OFFSET(5) NUMBITS(1) [],
            // Port overcurrent active
            POCA OFFSET(4) NUMBITS(1) [],
            // Port enable/disable change
            PENCHNG OFFSET(3) NUMBITS(1) [],
            // Port enable
            PENA OFFSET(2) NUMBITS(1) [],
            // Port connect detected
            PCDET OFFSET(1) NUMBITS(1) [],
            // Port connect status
            PCSTS OFFSET(0) NUMBITS(1) []
    ],


    HCCHARx [
            // Channel enable
            CHENA OFFSET(31) NUMBITS(1) [],
            // Channel disable
            CHDIS OFFSET(30) NUMBITS(1) [],
            // Odd frame
            ODDFRM OFFSET(29) NUMBITS(1) [],
            // Device address
            DAD OFFSET(22) NUMBITS(7) [],
            // Multicount
            MCNT OFFSET(20) NUMBITS(2) [],
            // Endpoint type
            EPTYP OFFSET(18) NUMBITS(2) [],
            // Low-speed device
            LSDEV OFFSET(17) NUMBITS(1) [],
            // Endpoint direction
            EPDIR OFFSET(15) NUMBITS(1) [],
            // Endpoint number
            EPNUM OFFSET(11) NUMBITS(4) [],
            // Maximum packet size
            MPSIZ OFFSET(0) NUMBITS(11) []
    ],

    HCINTx [
            // Data toggle error
            DTERR OFFSET(10) NUMBITS(1) [],
            // Frame overrun
            FRMOR OFFSET(9) NUMBITS(1) [],
            // Babble error
            BBERR OFFSET(8) NUMBITS(1) [],
            // Transaction error
            TXERR OFFSET(7) NUMBITS(1) [],
            // ACK response received/transmitted interrupt
            ACK OFFSET(5) NUMBITS(1) [],
            // NAK response received interrupt
            NAK OFFSET(4) NUMBITS(1) [],
            // STALL response received interrupt
            STALL OFFSET(3) NUMBITS(1) [],
            // Channel halted
            CHH OFFSET(1) NUMBITS(1) [],
            // Transfer completed
            XFRC OFFSET(0) NUMBITS(1) []
    ],
    
    HCINTMSKx [
            // Data toggle error mask
            DTERRM OFFSET(10) NUMBITS(1) [],
            // Frame overrun mask
            FRMORM OFFSET(9) NUMBITS(1) [],
            // Babble error mask
            BBERRM OFFSET(8) NUMBITS(1) [],
            // Transaction error mask
            TXERRM OFFSET(7) NUMBITS(1) [],
            // ACK response received/transmitted interrupt mask
            ACKM OFFSET(5) NUMBITS(1) [],
            // NAK response received interrupt mask
            NAKM OFFSET(4) NUMBITS(1) [],
            // STALL response received interrupt mask
            STALLM OFFSET(3) NUMBITS(1) [],
            // Channel halted mask
            CHHM OFFSET(1) NUMBITS(1) [],
            // Transfer completed mask
            XFRCM OFFSET(0) NUMBITS(1) []
    ],

    HCTSIZx [
            // Do Ping
            DOPNG OFFSET(31) NUMBITS(1) [],
            // Data PID
            DPID OFFSET(29) NUMBITS(2) [],
            // Packet count
            PKTCNT OFFSET(19) NUMBITS(10) [],
            // Transfer size
            XFRSIZ OFFSET(0) NUMBITS(19) []
    ],
    
    // Device-mode registers

    DCFG [
            // Erratic error interrupt mask
            ERRATIM OFFSET(15) NUMBITS(1) [],
            // Periodic frame interval
            PFIVL OFFSET(11) NUMBITS(2) [],
            // Device address
            DAD OFFSET(4) NUMBITS(7) [],
            // Non-zero-length status OUT handshake
            NZLSOHSK OFFSET(2) NUMBITS(1) [],
            // Device speed
            DSPD OFFSET(0) NUMBITS(2) []
    ],


    DCTL [
            // Deep sleep BESL reject
            DSBESLRJCT OFFSET(18) NUMBITS(1) [],
            // Power-on programming done
            POPRGDNE OFFSET(11) NUMBITS(1) [],
            // Clear global OUT NAK
            CGONAK OFFSET(10) NUMBITS(1) [],
            // Set global OUT NAK
            SGONAK OFFSET(9) NUMBITS(1) [],
            // Clear global IN NAK
            CGINAK OFFSET(8) NUMBITS(1) [],
            // Set global IN NAK
            SGINAK OFFSET(7) NUMBITS(1) [],
            // Test control
            TCTL OFFSET(4) NUMBITS(3) [],
            // Global OUT NAK status
            GONSTS OFFSET(3) NUMBITS(1) [],
            // Global IN NAK status
            GINSTS OFFSET(2) NUMBITS(1) [],
            // Soft disconnect
            SDIS OFFSET(1) NUMBITS(1) [],
            // Remote wakeup signaling
            RWUSIG OFFSET(0) NUMBITS(1) []
    ],

    DSTS [
            // Device line status
            DEVLNSTS OFFSET(22) NUMBITS(2) [],
            // Frame number of the received SOF
            FNSOF OFFSET(8) NUMBITS(14) [],
            // Erratic error
            EERR OFFSET(3) NUMBITS(1) [],
            // Enumerated speed
            ENUMSPD OFFSET(1) NUMBITS(2) [],
            // Suspend status
            SUSPSTS OFFSET(0) NUMBITS(1) []
    ],


    DIEPMSK [
            // NAK interrupt mask
            NAKM OFFSET(13) NUMBITS(1) [],
            // IN endpoint NAK effective mask
            INEPNEM OFFSET(6) NUMBITS(1) [],
            // IN token received with EP mismatch mask
            INEPNMM OFFSET(5) NUMBITS(1) [],
            // IN token received when Tx FIFO empty mask
            ITTXFEMSK OFFSET(4) NUMBITS(1) [],
            // Timeout condition mask (Non-isochronous endpoints)
            TOM OFFSET(3) NUMBITS(1) [],
            // Endpoint disabled interrupt mask
            EPDM OFFSET(1) NUMBITS(1) [],
            // Transfer completed interrupt mask
            XFRCM OFFSET(0) NUMBITS(1) []
    ],

    DOEPMSK [
            // NAK interrupt mask
            NAKMSK OFFSET(13) NUMBITS(1) [],
            // Babble error interrupt mask
            BERRM OFFSET(12) NUMBITS(1) [],
            // Out packet error mask
            OUTPKTERRM OFFSET(8) NUMBITS(1) [],
            // Status phase received for control write mask
            STSPHSRXM OFFSET(5) NUMBITS(1) [],
            // OUT token received when endpoint disabled mask. 
            // Applies to control OUT endpoints only
            OTEPDM OFFSET(4) NUMBITS(1) [],
            // STUPM: SETUP phase done mask. Applies to control endpoints only
            STUPM OFFSET(3) NUMBITS(1) [],
            // Endpoint disabled interrupt mask
            EPDM OFFSET(1) NUMBITS(1) [],
            // Transfer completed interrupt mask
            XFRCM OFFSET(0) NUMBITS(1) []
    ],

    DAINT [
            // OUT endpoint interrupt bits
            OEPINT OFFSET(16) NUMBITS(16) [],
            // IN endpoint interrupt bits
            IEPINT OFFSET(0) NUMBITS(16) []
    ],

    DAINTMSK [
            // OUT EP interrupt mask bits
            OEPM OFFSET(16) NUMBITS(16) [],
            // IN EP interrupt mask bits
            IEPM OFFSET(0) NUMBITS(16) []
    ],

    DVBUSDIS [
            // Device V BUS discharge time
            VBUSDT OFFSET(0) NUMBITS(16) []
    ],

    DVBUSPULSE [
            // Device V BUS pulsing time. This feature is only relevant to OTG1.3
            DVBUSP OFFSET(0) NUMBITS(16) []
    ],

    DIEPEMPMSK [
            // IN EP Tx FIFO empty interrupt mask bits
            INEPTXFEM OFFSET(0) NUMBITS(16) []
    ],

    DIEPCTL0 [
            // Endpoint enable
            EPENA OFFSET(31) NUMBITS(1) [],
            // Endpoint disable
            EPDIS OFFSET(30) NUMBITS(1) [],
            // Set NAK
            SNAK OFFSET(27) NUMBITS(1) [],
            // Clear NAK
            CNAK OFFSET(26) NUMBITS(1) [],
            // Tx FIFO number
            TXFNUM OFFSET(22) NUMBITS(4) [],
            // STALL handshake
            STALL OFFSET(21) NUMBITS(1) [],
            // Endpoint type
            EPTYP OFFSET(18) NUMBITS(2) [],
            // NAK status
            NAKSTS OFFSET(17) NUMBITS(1) [],
            // USB active endpoint
            USBAEP OFFSET(15) NUMBITS(1) [],
            // Maximum packet size
            MPSIZ OFFSET(0) NUMBITS(2) []
    ],

    DIEPCTLx [
            // Endpoint enable
            EPENA OFFSET(31) NUMBITS(1) [],
            // Endpoint disable
            EPDIS OFFSET(30) NUMBITS(1) [],
            // Set odd frame
            SODDFRM OFFSET(29) NUMBITS(1) [],
            // Set DATA0 PID
            SD0PID OFFSET(28) NUMBITS(1) [],
            // Set even frame
            SEVNFRM OFFSET(28) NUMBITS(1) [],
            // Set NAK
            SNAK OFFSET(27) NUMBITS(1) [],
            // Clear NAK
            CNAK OFFSET(26) NUMBITS(1) [],
            // Tx FIFO number
            TXFNUM OFFSET(22) NUMBITS(4) [],
            // STALL handshake
            STALL OFFSET(21) NUMBITS(1) [],
            // Endpoint type
            EPTYP OFFSET(18) NUMBITS(2) [],
            // NAK status
            NAKSTS OFFSET(17) NUMBITS(1) [],
            // Even/odd frame
            EONUM OFFSET(16) NUMBITS(1) [],
            // Endpoint data PID
            DPID OFFSET(16) NUMBITS(1) [],
            // USB active endpoint
            USBAEP OFFSET(15) NUMBITS(1) [],
            // Maximum packet size
            MPSIZ OFFSET(0) NUMBITS(11) []
    ],

    DIEPINTx [
            // NAK input
            NAK OFFSET(13) NUMBITS(1) [],
            // Packet dropped status
            PKTDRPSTS OFFSET(11) NUMBITS(1) [],
            // Transmit FIFO empty
            TXFE OFFSET(7) NUMBITS(1) [],
            // IN endpoint NAK effective
            INEPNE OFFSET(6) NUMBITS(1) [],
            // IN token received with EP mismatch
            INEPNM OFFSET(5) NUMBITS(1) [],
            // IN token received when Tx FIFO is empty
            ITTXFE OFFSET(4) NUMBITS(1) [],
            // Timeout condition
            TOC OFFSET(3) NUMBITS(1) [],
            // Endpoint disabled interrupt
            EPDISD OFFSET(1) NUMBITS(1) [],
            // Transfer completed interrupt
            XFRC OFFSET(0) NUMBITS(1) []
    ],

    DIEPTSIZ0 [
            // Packet count
            PKTCNT OFFSET(19) NUMBITS(2) [],
            // Transfer size
            XFRSIZ OFFSET(0) NUMBITS(7) []
    ],

    DTXFSTSx [
            // IN endpoint Tx FIFO space available
            INEPTFSAV OFFSET(0) NUMBITS(16) []
    ],

    DIEPTSIZx [
            // Multi count
            MCNT OFFSET(29) NUMBITS(2) [],
            // Packet count
            PKTCNT OFFSET(19) NUMBITS(10) [],
            // Transfer size
            XFRSIZ OFFSET(0) NUMBITS(19) []
    ],

    DOEPCTL0 [
            // Endpoint enable
            EPENA OFFSET(31) NUMBITS(1) [],
            // Endpoint disable
            EPDIS OFFSET(30) NUMBITS(1) [],
            // Set NAK
            SNAK OFFSET(27) NUMBITS(1) [],
            // Clear NAK
            CNAK OFFSET(26) NUMBITS(1) [],
            // STALL handshake
            STALL OFFSET(21) NUMBITS(1) [],
            // Snoop mode
            SNPM OFFSET(20) NUMBITS(1) [],
            // Endpoint type
            EPTYP OFFSET(18) NUMBITS(2) [],
            // NAK status
            NAKSTS OFFSET(17) NUMBITS(1) [],
            // USB active endpoint
            USBAEP OFFSET(15) NUMBITS(1) [],
            // Maximum packet size
            MPSIZ OFFSET(0) NUMBITS(2) []
    ],

    DOEPINTx [
            // NAK input
            NAK OFFSET(13) NUMBITS(1) [],
            // Babble error interrupt
            BERR OFFSET(12) NUMBITS(1) [],
            // Status phase received for control write
            STSPHSRX OFFSET(5) NUMBITS(1) [],
            // OUT token received when endpoint disabled
            OTEPDIS OFFSET(4) NUMBITS(1) [],
            // SETUP phase done
            STUP OFFSET(3) NUMBITS(1) [],
            // Endpoint disabled interrupt
            EPDISD OFFSET(1) NUMBITS(1) [],
            // Transfer completed interrupt
            XFRC OFFSET(0) NUMBITS(1) []
    ],

    DOEPTSIZ0 [
            // SETUP packet count
            STUPCNT OFFSET(29) NUMBITS(2) [],
            // Packet count
            PKTCNT OFFSET(19) NUMBITS(1) [],
            // Transfer size
            XFRSIZ OFFSET(0) NUMBITS(7) []
    ],

    DOEPCTLx [
            // Endpoint enable
            EPENA OFFSET(31) NUMBITS(1) [],
            // Endpoint disable
            EPDIS OFFSET(30) NUMBITS(1) [],
            // Set DATA1 PID
            SD1PID OFFSET(29) NUMBITS(1) [],
            // Set odd frame
            SODDFRM OFFSET(29) NUMBITS(1) [],
            // Set DATA0 PID
            SD0PID OFFSET(28) NUMBITS(1) [],
            // Set even frame
            SEVNFRM OFFSET(28) NUMBITS(1) [],
            // Set NAK
            SNAK OFFSET(27) NUMBITS(1) [],
            // Clear NAK
            CNAK OFFSET(26) NUMBITS(1) [],
            // STALL handshake
            STALL OFFSET(21) NUMBITS(1) [],
            // Snoop mode
            SNPM OFFSET(20) NUMBITS(1) [],
            // Endpoint type
            EPTYP OFFSET(18) NUMBITS(2) [],
            // NAK status
            NAKSTS OFFSET(17) NUMBITS(1) [],
            // Even/odd frame
            EONUM OFFSET(16) NUMBITS(1) [],
            // Endpoint data PID
            DPID OFFSET(16) NUMBITS(1) [],
            // USB active endpoint
            USBAEP OFFSET(15) NUMBITS(1) [],
            // Maximum packet size
            MPSIZ OFFSET(0) NUMBITS(11) []
    ],

    DOEPTSIZx [
            // Received data PID
            RXDPID OFFSET(29) NUMBITS(2) [],
            // SETUP packet count
            STUPCNT OFFSET(29) NUMBITS(2) [],
            // Packet count
            PKTCNT OFFSET(19) NUMBITS(10) [],
            // Transfer size
            XFRSIZ OFFSET(0) NUMBITS(19) []
    ],

    PCGCCTL [
            // Deep Sleep
            SUSP OFFSET(7) NUMBITS(1) [],
            // PHY in Sleep
            PHYSLEEP OFFSET(6) NUMBITS(1) [],
            // Enable sleep clock gating
            ENL1GTG OFFSET(5) NUMBITS(1) [],
            // PHY suspended
            PHYSUSP OFFSET(4) NUMBITS(1) [],
            // Gate HCLK
            GATEHCLK OFFSET(1) NUMBITS(1) [],
            // Stop PHY clock
            STPPCLK OFFSET(0) NUMBITS(1) []
    ]
];

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UsbState {
    Disabled,
    Started,
    Initialized,
    PoweredOn,
    Attached,
    Configured,
}

#[derive(Copy, Clone, Debug)]
pub enum EndpointState {
    Disabled,
    Ctrl(CtrlState),
    Bulk(Option<BulkInState>, Option<BulkOutState>),
    Interrupt(u32, InterruptState),
}

/// State of the control endpoint (endpoint 0).
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum CtrlState {
    /// Control endpoint is idle, and waiting for a command from the host.
    Init,
    /// Control endpoint has started an IN transfer.
    ReadIn,
    /// Control endpoint has moved to the status phase.
    ReadStatus,
    /// Control endpoint is handling a control write (OUT) transfer.
    WriteOut,
    /// Control endpoint needs to set the address in hardware
    SetAddress,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BulkInState {
    // The endpoint is ready to perform transactions.
    Init,
    // There is a pending IN packet transfer on this endpoint.
    In(usize),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum BulkOutState {
    // The endpoint is ready to perform transactions.
    Init,
    // There is a pending OUT packet in this endpoint's buffer, to be read by
    // the client application.
    OutDelay,
    // There is a pending EPDATA to reply to.
    OutData(usize),
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum InterruptState {
    // The endpoint is ready to perform transactions.
    Init,
    // There is a pending IN packet transfer on this endpoint.
    In(usize),
}

pub struct Endpoint<'a> {
    slice_in: OptionalCell<&'a [VolatileCell<u8>]>,
    slice_out: OptionalCell<&'a [VolatileCell<u8>]>,
    state: Cell<EndpointState>,
    transfer_size: Cell<u8>,
    request_type: Cell<u8>
}


impl Endpoint<'_> {
    pub const fn new() -> Self {
        Endpoint {
            slice_in: OptionalCell::empty(),
            slice_out: OptionalCell::empty(),
            state: Cell::new(EndpointState::Disabled),
            transfer_size: Cell::new(0),
            request_type: Cell::new(0)
        }
    }

}

const USBOTG_BASE: StaticRef<UsbotgRegisters> =
    unsafe { StaticRef::new(0x50000000 as *const UsbotgRegisters) };

const RECEIVE_FIFO: StaticRef<[VolatileCell<u32>; 128]> =
    unsafe { StaticRef::new(0x50001000 as *const [VolatileCell<u32>; 128]) };

const TRANSMIT_FIFO_0: StaticRef<[VolatileCell<u32>; 64]> =
    unsafe { StaticRef::new(0x50001080 as *const [VolatileCell<u32>; 64]) };

const TRANSMIT_FIFO_1: StaticRef<[VolatileCell<u32>; 128]> =
    unsafe { StaticRef::new(0x500010C0 as *const [VolatileCell<u32>; 128]) };    

const STS_DATA_UPDT: u32 = 2;
const STS_SETUP_UPDT: u32 = 6;
const STS_SETUP_COMPLETE: u32 = 4;

const USB_DEVICE_MODE: u32 = 0;
const USB_HOST_MODE: u32 = 1;

const USBD_HS_SPEED: u32 = 0;
const USBD_FS_SPEED: u32 = 2;

const USB_OTG_SPEED_FULL: u32 = 3;

const DCFG_FRAME_INTERVAL_80: u32 = 0;

const USB_OTG_CORE_ID_300A: u32 = 0x4F54300A as u32;
const USB_OTG_CORE_ID_310A: u32 = 0x4F54310A as u32;

const DSTS_ENUMSPD_HS_PHY_30MHZ_OR_60MHZ: u32 = 0;
const DSTS_ENUMSPD_FS_PHY_30MHZ_OR_60MHZ: u32 = 1;
const DSTS_ENUMSPD_FS_PHY_48MHZ: u32 = 3;

const NUM_ENDPOINTS: usize = 6;

pub struct Usbotg<'a> {
    registers: StaticRef<UsbotgRegisters>,
    state: OptionalCell<UsbState>,
    client: OptionalCell<&'a dyn hil::usb::Client<'a>>,
    descriptors: [Endpoint<'a>; NUM_ENDPOINTS],
    setup_buffer: OptionalCell<&'a [VolatileCell<u8>]>,
    clock: USBOTGClock<'a>,
    addr: Cell<u16>,
    receive_fifo: StaticRef<[VolatileCell<u32>; 128]>,
    transmit_fifo_0: StaticRef<[VolatileCell<u32>; 64]>,
    transmit_fifo_1: StaticRef<[VolatileCell<u32>; 128]>,
}

impl<'a> Usbotg<'a> {

    pub const fn new(rcc: &'a rcc::Rcc) -> Self {
        Usbotg {
            registers: USBOTG_BASE,
            state: OptionalCell::new(UsbState::Disabled),
            client: OptionalCell::empty(),
            descriptors: [
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
                Endpoint::new(),
            ],
            setup_buffer: OptionalCell::empty(),
            clock: USBOTGClock(rcc::PeripheralClock::new(
                               rcc::PeripheralClockType::AHB2(rcc::HCLK2::OTGFS),
                               rcc)),
            addr: Cell::new(0),
            receive_fifo: RECEIVE_FIFO,
            transmit_fifo_0: TRANSMIT_FIFO_0,
            transmit_fifo_1: TRANSMIT_FIFO_1,
        }
    }

    pub fn is_enabled_clock(&self) -> bool {
        self.clock.is_enabled()
    }

    pub fn enable_clock(&self) {
        self.clock.enable();
    }

    pub fn disable_clock(&self) {
        self.clock.disable();
    }
    pub fn get_state(&self) -> UsbState {
        self.state.expect("get_state: state value is in use")
    }

    fn set_state(&self, state: UsbState) {
        self.state.set(state);
    }

    fn usb_reset(&self){
        
        self.activate_endpoint_out(TransferType::Control, 0);
        self.activate_endpoint_in(TransferType::Control, 0);

        self.registers.dcfg.modify(DCFG::DSPD.val(3));
    }

    fn core_initialization(&self){

        self.registers.gusbcfg.modify(GUSBCFG::PHYSEL::SET);

        if self.core_reset() == false{
                panic!("Core reset failed.");
        }

        self.registers.gccfg.modify(GCCFG::PWRDWN::SET);
    }

    // TO REVIEW HERE
    // Check how to add HCLK and SPEED
    fn set_turnaround_time(&self){

        // will be hardcoding this since we're only gonna use 48Mhz
        self.registers.gusbcfg.modify(GUSBCFG::TRDT.val(6));

    }

    fn enable_global_interrupt(&self){
        self.registers.gahbcfg.modify(GAHBCFG::GINTMSK::SET);
    }

    fn disable_global_interrupt(&self){
        self.registers.gahbcfg.modify(GAHBCFG::GINTMSK::CLEAR);
    }

    fn set_current_mode(&self, usb_mode: u32){
        self.registers.gusbcfg.modify(GUSBCFG::FDMOD::CLEAR +
                                      GUSBCFG::FHMOD::CLEAR);

        if usb_mode == USB_HOST_MODE{
                self.registers.gusbcfg.modify(GUSBCFG::FHMOD::SET);
        }

        else{
            self.registers.gusbcfg.modify(GUSBCFG::FDMOD::SET);
        }

        // Delay for about 50 ms
        for _ in 0..650000 {
            cortexm4::support::nop();
        }

    }

    fn device_initialization(&self){
       
        self.registers.dieptxf0.set(0);
        
        for i in 1..NUM_ENDPOINTS{
            self.registers.dieptxfx[i-1].set(0);
        }

        self.registers.dctl.modify(DCTL::SDIS::SET);

         /* Deactivate VBUS Sensing B */
        self.registers.gccfg.modify(GCCFG::VBDEN::CLEAR);

        /* B-peripheral session valid override enable */
        self.registers.gotgctl.modify(GOTGCTL::BVALOEN::SET +
                                      GOTGCTL::BVALOVAL::SET);

        /* Restart the Phy Clock */ 
        self.registers.pcgcctl.set(0);

        /* Device mode configuration */
        self.registers.dcfg.modify(DCFG::PFIVL.val(DCFG_FRAME_INTERVAL_80));

        /* Set Core speed to Full speed mode */
        self.set_device_speed(USB_OTG_SPEED_FULL);

        /* Flush the TX FIFO */
        if self.flush_tx_fifo(0x10 as u32) == false{
            debug!("TX FIFO flush failed.");
        }

        /* Flush the RX FIFO */
        if self.flush_rx_fifo() == false{
            debug!("RX FIFO flush failed.");
        }

        /* Clear all pending Device Interrupts */
        self.registers.diepmsk.set(0);
        self.registers.doepmsk.set(0);
        self.registers.daintmsk.set(0);

        for i in 0..NUM_ENDPOINTS{

            // Endpoint 0 IN and OUT
            if i == 0{
                // Endpoint 0 IN

                if self.registers.endpoint0_in_registers.diepctl0.is_set(DIEPCTL0::EPENA){
                    self.registers.endpoint0_in_registers.diepctl0.set(0);
                    self.registers.endpoint0_in_registers.diepctl0.modify(DIEPCTL0::SNAK::SET);
                }

                else{
                    self.registers.endpoint0_in_registers.diepctl0.set(0);
                }

                self.registers.endpoint0_in_registers.dieptsiz0.set(0);
                self.registers.endpoint0_in_registers.diepint0.set(0xFB7F as u32);

                // Endpoint 0 OUT

                if self.registers.endpoint0_out_registers.doepctl0.is_set(DOEPCTL0::EPENA){
                    self.registers.endpoint0_out_registers.doepctl0.set(0);
                    self.registers.endpoint0_out_registers.doepctl0.modify(DOEPCTL0::SNAK::SET);
                }

                else{
                    self.registers.endpoint0_out_registers.doepctl0.set(0);
                }

                self.registers.endpoint0_out_registers.doeptsiz0.set(0);
                self.registers.endpoint0_out_registers.doepint0.set(0xFB7F as u32);

            }

            // Endpoints 1..5 IN and OUT
            else{

                // Endpoint i IN

                if self.registers.endpoint_in_registers[(i-1) as usize].diepctlx.is_set(DIEPCTLx::EPENA){
                    self.registers.endpoint_in_registers[(i-1) as usize].diepctlx.set(0);
                    self.registers.endpoint_in_registers[(i-1) as usize].diepctlx.modify(DIEPCTLx::EPDIS::SET + 
                                                                                         DIEPCTLx::SNAK::SET);
                }

                else{
                    self.registers.endpoint_in_registers[(i-1) as usize].diepctlx.set(0);
                }

                self.registers.endpoint_in_registers[(i-1) as usize].dieptsizx.set(0);
                self.registers.endpoint_in_registers[(i-1) as usize].diepintx.set(0xFB7F as u32);


                // Endpoint i OUT

                if self.registers.endpoint_out_registers[(i-1) as usize].doepctlx.is_set(DOEPCTLx::EPENA){
                    self.registers.endpoint_out_registers[(i-1) as usize].doepctlx.set(0);
                    self.registers.endpoint_out_registers[(i-1) as usize].doepctlx.modify(DOEPCTLx::EPDIS::SET +
                                                                                          DOEPCTLx::SNAK::SET);
                }

                else{
                    self.registers.endpoint_out_registers[(i-1) as usize].doepctlx.set(0);
                }

                self.registers.endpoint_out_registers[(i-1) as usize].doeptsizx.set(0);
                self.registers.endpoint_out_registers[(i-1) as usize].doepintx.set(0xFB7F as u32);
            }

        }

        /* Disable all interrupts. */
        self.registers.gintmsk.set(0);

        /* Clear any pending interrupts */
        self.registers.gintsts.set(0xBFFFFFFF as u32);

        /* Enable the common interrupts */
        self.registers.gintmsk.modify(GINTMSK::RXFLVLM::SET +
                                      GINTMSK::USBSUSPM::SET +
                                      GINTMSK::USBRST::SET +
                                      GINTMSK::ENUMDNEM::SET +
                                      GINTMSK::IEPINT::SET +
                                      GINTMSK::OEPINT::SET +
                                      GINTMSK::IISOIXFRM::SET +
                                      GINTMSK::IISOOXFRM::SET +
                                      GINTMSK::WUIM::SET);
  
    }

    fn flush_tx_fifo(&self, num: u32) -> bool{

        let mut count: u32 = 0;

        self.registers.grstctl.set(0);
        self.registers.grstctl.modify(GRSTCTL::TXFFLSH::SET +
                                      GRSTCTL::TXFNUM.val(num));
        
        loop{
                count+= 1;

                if count > 200000{
                        return false;
                }

                if !self.registers.grstctl.is_set(GRSTCTL::TXFFLSH){
                        break;
                }
        }

        true
    }

    fn flush_rx_fifo(&self) -> bool{

        let mut count: u32 = 0;

        self.registers.grstctl.set(0);
        self.registers.grstctl.modify(GRSTCTL::RXFFLSH::SET);

        loop{
                count+= 1;
                if count > 200000{
                        return false;
                }

                if !self.registers.grstctl.is_set(GRSTCTL::RXFFLSH){
                        break;
                }
        }

        true
    }

    fn set_device_speed(&self, speed: u32){
        self.registers.dcfg.modify(DCFG::DSPD.val(speed));
    }

    fn get_device_speed(&self) -> u32{

        let dev_enum_speed: u32 = self.registers.dsts.read(DSTS::ENUMSPD);

        if dev_enum_speed == DSTS_ENUMSPD_HS_PHY_30MHZ_OR_60MHZ{
            return USBD_HS_SPEED;
        }

        else if dev_enum_speed == DSTS_ENUMSPD_FS_PHY_30MHZ_OR_60MHZ || dev_enum_speed == DSTS_ENUMSPD_FS_PHY_48MHZ{
            return USBD_FS_SPEED;
        }

        else{
            return 0xF as u32;
        }
    }

    fn activate_endpoint_in(&self, transfer_type: TransferType, endpoint: usize){

        self.registers.daintmsk.modify(DAINTMSK::IEPM.val(self.registers.daintmsk.read(DAINTMSK::IEPM) | 2_u32.pow(endpoint as u32)));

        if endpoint == 0 {

            if self.registers.endpoint0_in_registers.diepctl0.read(DIEPCTL0::USBAEP) == 0{
                
                self.registers.endpoint0_in_registers.diepctl0.modify(
                                                                      DIEPCTL0::MPSIZ.val(0) +
                                                                      DIEPCTL0::TXFNUM.val(endpoint as u32) +
                                                                      DIEPCTL0::EPENA::SET);

                }

            self.descriptors[endpoint].state.set(EndpointState::Ctrl(CtrlState::Init));
        }
        
        else{

            let mut ep_type = 0;

            match transfer_type{         
                TransferType::Control => {ep_type = 0; self.descriptors[endpoint].state.set(EndpointState::Ctrl(CtrlState::Init));},
                //TransferType::Isochronous => {ep_type = 1; self.descriptors[endpoint].state.set(EndpointState::Iso);},
                TransferType::Isochronous => {panic!("Isochronous transfers are not supported yet.");},
                TransferType::Bulk => {ep_type = 2; self.descriptors[endpoint].state.set(EndpointState::Bulk(Some(BulkInState::Init), None));},
                TransferType::Interrupt => {ep_type = 3; self.descriptors[endpoint].state.set(EndpointState::Interrupt(64, InterruptState::Init));},
            };

               
            if self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.read(DIEPCTLx::USBAEP) == 0{
                    self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.modify(
                                                                        DIEPCTLx::MPSIZ.val(0) + 
                                                                        DIEPCTLx::EPTYP.val(ep_type) +
                                                                        DIEPCTLx::TXFNUM.val(endpoint as u32) +
                                                                        DIEPCTLx::SD0PID::SET +
                                                                        DIEPCTLx::USBAEP::SET);

            }

        }
    }

    fn activate_endpoint_out(&self, transfer_type: TransferType, endpoint: usize){

        self.registers.daintmsk.modify(DAINTMSK::OEPM.val(self.registers.daintmsk.read(DAINTMSK::OEPM) | 2_u32.pow(endpoint as u32)));

        if endpoint == 0 {

            if self.registers.endpoint0_out_registers.doepctl0.read(DOEPCTL0::USBAEP) == 0{

                self.registers.endpoint0_out_registers.doepctl0.modify(DOEPCTL0::EPENA::SET);

            }

            self.descriptors[endpoint].state.set(EndpointState::Ctrl(CtrlState::Init));
        }
                       
        else{

            let mut ep_type = 0;

            match transfer_type{
                TransferType::Control => {ep_type = 0; self.descriptors[endpoint].state.set(EndpointState::Ctrl(CtrlState::Init));},
                //TransferType::Isochronous => {ep_type = 1; self.descriptors[endpoint].state.set(EndpointState::Iso);},
                TransferType::Isochronous => {panic!("Isochronous transfers are not supported yet.");},
                TransferType::Bulk => {ep_type = 2; self.descriptors[endpoint].state.set(EndpointState::Bulk(Some(BulkInState::Init), None));},
                TransferType::Interrupt => {ep_type = 3; self.descriptors[endpoint].state.set(EndpointState::Interrupt(64, InterruptState::Init));},
            };

            if self.registers.endpoint_out_registers[(endpoint - 1)].doepctlx.read(DOEPCTLx::USBAEP) == 0{
                self.registers.endpoint_out_registers[(endpoint - 1)].doepctlx.modify(
                                                                                DOEPCTLx::MPSIZ.val(0) + 
                                                                                DOEPCTLx::EPTYP.val(ep_type) +
                                                                                DOEPCTLx::SD0PID::SET +
                                                                                DOEPCTLx::USBAEP::SET);

                    }
            }
    }

    fn ep_0_start_transfer_in(&self){
        
            if self.descriptors[0].transfer_size.get() == 0{
                self.registers.endpoint0_in_registers.dieptsiz0.modify(DIEPTSIZ0::PKTCNT::CLEAR +
                                                                       DIEPTSIZ0::PKTCNT.val(1) +
                                                                       DIEPTSIZ0::XFRSIZ::CLEAR);
            }
            
            else{
                self.registers.endpoint0_in_registers.dieptsiz0.modify(DIEPTSIZ0::PKTCNT::CLEAR +
                                                                       DIEPTSIZ0::XFRSIZ::CLEAR);

                if self.descriptors[0].transfer_size.get() > 64{
                    self.descriptors[0].transfer_size.set(64);
                }

                self.registers.endpoint0_in_registers.dieptsiz0.modify(DIEPTSIZ0::PKTCNT.val(1) +
                                                                       DIEPTSIZ0::XFRSIZ.val(self.descriptors[0].transfer_size.get() as u32));
            }

            self.registers.endpoint0_in_registers.diepctl0.modify(DIEPCTL0::CNAK::SET +
                                                                  DIEPCTL0::EPENA::SET);

            if self.descriptors[0].transfer_size.get() > 0{
                self.registers.diepempmsk.set(self.registers.diepempmsk.get() | 2_u32.pow(0 as u32));
            }

    }

    fn ep_0_start_transfer_out(&self){

        self.registers.endpoint0_out_registers.doeptsiz0.modify(DOEPTSIZ0::XFRSIZ::CLEAR +
            DOEPTSIZ0::PKTCNT::CLEAR);

        if self.descriptors[0].transfer_size.get() > 0{
        self.descriptors[0].transfer_size.set(64);
        }

        self.registers.endpoint0_out_registers.doeptsiz0.modify(DOEPTSIZ0::PKTCNT.val(1) +
                    DOEPTSIZ0::XFRSIZ.val(64));

        self.registers.endpoint0_out_registers.doepctl0.modify(DOEPCTL0::CNAK::SET +
                DOEPCTL0::EPENA::SET);
    }

    fn write_packet(&self, ep: usize, len: usize){

        debug!("ep = {}", ep);

        let ep_buffer = self.descriptors[ep].slice_in.expect("No IN slice set for this descriptor");
        
        let mut buffer_cell_start: usize = 0;
        let mut buffer_cell_end: usize = 4;
        let mut data_packet_cell: usize = 0;

        if ep_buffer.len() < 8{
            panic!("Buffer length cannot be less than 8.");
        }

        let packets_to_write = (len + 3) / 4;

        for x in 0..packets_to_write{
            debug!("TxFIFO init packet {} - {}", x, self.transmit_fifo_0[x as usize].get());
        }

        for i in 0..packets_to_write{

            let mut packet = [0; 4];

            for j in buffer_cell_start..buffer_cell_end{

                packet[(j - buffer_cell_start)as usize] = ep_buffer[j as usize].get();

            }

            //debug!("Packet {} - {:?}", i, packet);
            let data: u32 = self.bytes_to_data_packet(packet);
            debug!("data {} - {}", i, data);

            if ep == 0{
                //debug!("data = {:X}", data);
                self.transmit_fifo_0[i as usize].set(data);
            }

            else if ep == 1{
                self.transmit_fifo_1[i as usize].set(data);
            }

            buffer_cell_start = buffer_cell_end;
            buffer_cell_end += 4;
        }

        for x in 0..packets_to_write{
            debug!("TxFIFO written packet {} - {}", x, self.transmit_fifo_0[x as usize].get());
        }

    }

    fn read_packet(&self, ep: usize, len: u32){

        self.clear_endpoint_out_buffer(ep);

        let ep_buffer = self.descriptors[ep].slice_out.expect("No OUT slice set for this descriptor");
        self.descriptors[ep].transfer_size.set(len as u8);

        let mut buffer_cell_start: usize = 0;
        let mut buffer_cell_end: usize = 0;
        let mut data_packet_cell: usize = 0;

        if ep_buffer.len() < 8{
            panic!("Buffer length cannot be less than 8.");
        }

        let packets_to_read = (len + 3) / 4;

        for i in 0..packets_to_read{

            let data_packet = self.data_packet_to_bytes(self.receive_fifo[i as usize].get());

            buffer_cell_end += data_packet.len();

            for j in buffer_cell_start..buffer_cell_end{

                ep_buffer[j as usize].set(data_packet[data_packet_cell]);
                //debug!("{} - {:X}", j, data_packet[data_packet_cell]);

                data_packet_cell += 1;
            }

            data_packet_cell = 0;
            buffer_cell_start = buffer_cell_end;

        }
        // 0 = HostToDevice
        // 1 = DeviceToHost
        if ep_buffer[0].get() & (1 << 7) != 0{
            self.descriptors[ep].request_type.set(1);
        }

    }

    fn clear_endpoint_out_buffer(&self, ep: usize){
        let ep_buffer = self.descriptors[ep].slice_out.expect("No OUT slice set for this descriptor");

        for i in 0..8{
            ep_buffer[i as usize].set(0);
        }
    }

    fn clear_endpoint_in_buffer(&self, ep: usize){
        let ep_buffer = self.descriptors[ep].slice_in.expect("No OUT slice in for this descriptor");

        for i in 0..8{
            ep_buffer[i as usize].set(0);
        }
    }

    fn data_packet_to_bytes(&self, data: u32) -> [u8; 4]{

        let data_packet_bytes = data.to_ne_bytes();

        data_packet_bytes
    }

    fn bytes_to_data_packet(&self, bytes: [u8; 4]) -> u32{
        ((bytes[0] as u32) << 24) +
        ((bytes[1] as u32) << 16) +
        ((bytes[2] as u32) <<  8) +
        ((bytes[3] as u32) <<  0)
    }


    fn ep_set_stall_in(&self, endpoint: usize){
        if endpoint == 0{
            self.registers.endpoint0_in_registers.diepctl0.modify(DIEPCTL0::STALL::SET);
        }

        else{
            if !self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.is_set(DIEPCTLx::EPENA){
                self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.modify(DIEPCTLx::EPDIS::CLEAR);
            }
            self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.modify(DIEPCTLx::STALL::SET);
        }
    }

    fn ep_set_stall_out(&self, endpoint: usize){
        if endpoint == 0{
            self.registers.endpoint0_out_registers.doepctl0.modify(DOEPCTL0::STALL::SET);
        }

        else{
            if !self.registers.endpoint_out_registers[(endpoint - 1) as usize].doepctlx.is_set(DOEPCTLx::EPENA){
                self.registers.endpoint_out_registers[(endpoint - 1) as usize].doepctlx.modify(DOEPCTLx::EPDIS::CLEAR);
            }
            self.registers.endpoint_out_registers[(endpoint - 1) as usize].doepctlx.modify(DOEPCTLx::STALL::SET);
        }
    }

    fn ep_clear_stall_in(&self, endpoint: usize){
        if endpoint == 0{
            self.registers.endpoint0_in_registers.diepctl0.modify(DIEPCTL0::STALL::CLEAR);
        }

        else{
            self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.modify(DIEPCTLx::STALL::CLEAR);

            if self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.read(DIEPCTLx::EPTYP) == 3 || 
                self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.read(DIEPCTLx::EPTYP) == 2{
                self.registers.endpoint_in_registers[(endpoint - 1)].diepctlx.modify(DIEPCTLx::SD0PID::SET);
            }
        }
    }

    fn ep_clear_stall_out(&self, endpoint: usize){
        if endpoint == 0{
            self.registers.endpoint0_out_registers.doepctl0.modify(DOEPCTL0::STALL::CLEAR);
        }

        else{
            self.registers.endpoint_out_registers[(endpoint - 1)].doepctlx.modify(DOEPCTLx::STALL::CLEAR);

            if self.registers.endpoint_out_registers[(endpoint - 1)].doepctlx.read(DOEPCTLx::EPTYP) == 3 || 
                self.registers.endpoint_out_registers[(endpoint - 1)].doepctlx.read(DOEPCTLx::EPTYP) == 2{
                self.registers.endpoint_out_registers[(endpoint - 1)].doepctlx.modify(DOEPCTLx::SD0PID::SET);
            }
        }
    }

    
    fn stop_device(&self){

        for i in 0..NUM_ENDPOINTS{
            if i == 0{
                self.registers.endpoint0_in_registers.diepint0.set(0xFB7F as u32);
                self.registers.endpoint0_out_registers.doepint0.set(0xFB7F as u32);
            }

            else{
                self.registers.endpoint_in_registers[(i - 1) as usize].diepintx.set(0xFB7F as u32);
                self.registers.endpoint_out_registers[(i - 1) as usize].doepintx.set(0xFB7F as u32);
            }
        }

        self.registers.diepmsk.set(0);
        self.registers.doepmsk.set(0);
        self.registers.daintmsk.set(0);

        if !self.flush_rx_fifo(){
            debug!("RX FIFO Flush failed.");
        }

        if !self.flush_tx_fifo(0x10 as u32){
            debug!("TX FIFO Flush failed.");
        }
    }

    fn set_device_address(&self, address: u32){
        
        self.registers.dcfg.modify(DCFG::DAD::CLEAR);
        self.registers.dcfg.modify(DCFG::DAD.val(address));
    }

    fn device_connect(&self){
        self.registers.pcgcctl.modify(PCGCCTL::STPPCLK::CLEAR + 
                                      PCGCCTL::GATEHCLK::CLEAR);

        self.registers.dctl.modify(DCTL::SDIS::CLEAR);
    }

    fn device_disconnect(&self){
        self.registers.pcgcctl.modify(PCGCCTL::STPPCLK::CLEAR + 
                                      PCGCCTL::GATEHCLK::CLEAR);

        self.registers.dctl.modify(DCTL::SDIS::SET);
    }

    fn get_mode(&self) -> u32{
        return self.registers.gintsts.read(GINTSTS::CMOD);
    }

    fn activate_setup(&self){
        self.registers.endpoint0_in_registers.diepctl0.modify(DIEPCTL0::MPSIZ.val(0));
        self.registers.dctl.modify(DCTL::CGINAK::SET);
    }

    fn ep0_out_start(&self){

        if (self.registers.cid.get()) > USB_OTG_CORE_ID_300A{
                if self.registers.endpoint0_out_registers.doepctl0.is_set(DOEPCTL0::EPENA){
                        return;
                }
        }

        self.registers.endpoint0_out_registers.doeptsiz0.set(0 as u32);

        self.registers.endpoint0_out_registers.doeptsiz0.modify(
                                        DOEPTSIZ0::PKTCNT.val(1) +
                                        DOEPTSIZ0::XFRSIZ.val(24) +
                                        DOEPTSIZ0::STUPCNT.val(3));
    }

    fn core_reset(&self) -> bool{
        let mut count: u32 = 0;

        loop{
                count += 1;

                if count > 200000{
                        return false;
                }

                if self.registers.grstctl.is_set(GRSTCTL::AHBIDL){
                        break;
                }
        }

        count = 0;

        self.registers.grstctl.modify(GRSTCTL::CSRST::SET);

        loop{
            count += 1;

            if count > 200000{
                    return false;
            }

            if !self.registers.grstctl.is_set(GRSTCTL::CSRST){
                    return true;
            }
        }
    }

    fn handle_complete_ctrl_status(&self){

        let endpoint: usize = 0;

        self.client.map(|client| {
            client.ctrl_status(endpoint);
            debug!("- task: ep0status");
            client.ctrl_status_complete(endpoint);
            self.descriptors[endpoint]
                .state
                .set(EndpointState::Ctrl(CtrlState::Init));
        });

    }

    fn handle_ep0_setup(&self){

        let endpoint: usize = 0;
        let state = self.descriptors[endpoint].state.get();

        let packet_size = self.descriptors[endpoint].transfer_size.get();

        self.client.map(|client|{

        match client.ctrl_setup(endpoint){
                hil::usb::CtrlSetupResult::OkSetAddress => {}
                hil::usb::CtrlSetupResult::Ok => {
                    // Setup request is successful.
                    if packet_size == 0 {
                        // Directly handle a 0 length setup request.
                        self.complete_ctrl_status();
                    } else {
                        debug!("NOT COMPLETE CTRL STATUS");
                    }
            }
            _ => {debug!("Unknown error");}

        }
        });
    }

    fn ep0_in_transfer(&self){

    }

    fn handle_ctrl_setup(&self) {

        let size = self.descriptors[0].transfer_size.get();
        let in_buffer = self.descriptors[0].slice_in.expect("No IN slice for this descriptor");

        self.client.map(|client|{

            match client.ctrl_setup(0){

                hil::usb::CtrlSetupResult::OkSetAddress => {debug!("Address set successfully. -> {}", self.registers.dcfg.read(DCFG::DAD));}
                hil::usb::CtrlSetupResult::Ok =>{

                    if size == 0{
                        debug!("Complete CTRL status");
                        self.complete_ctrl_status();
                    }

                    else{
                        if self.descriptors[0].request_type.get() == 1{
                            self.descriptors[0].state.set(EndpointState::Ctrl(CtrlState::ReadIn));

                            for i in 0..18{
                                debug!("BEFORE i - {}", in_buffer[i as usize].get());
                            }

                            match client.ctrl_in(0){
                                hil::usb::CtrlInResult::Packet(size, last) => {
                                    if size == 0 {
                                        internal_err!("Empty ctrl packet?");
                                    }
                                    if last{

                                        // for i in 0..18{
                                        //     debug!("AFTER i - {}", in_buffer[i as usize].get());
                                        // }

                                        self.descriptors[0].state.set(EndpointState::Ctrl(CtrlState::ReadStatus));
                                        debug!("CTRL IN completed successfully. Sending data to Tx FIFO");
                                        //self.write_packet(0, size);
                                        self.descriptors[0].transfer_size.set(size as u8);
                                        self.ep_0_start_transfer_in();
                                    }
                            }

                            hil::usb::CtrlInResult::Delay => {
                                internal_err!("Unexpected CtrlInResult::Delay");
                                // NAK is automatically sent by the modem.
                            }
            
                            hil::usb::CtrlInResult::Error => {
                                // An error occurred, we STALL
                                debug!("hil::usb::CtrlInResult::Error");
                                self.ep_set_stall_in(0);
                            }
                        }
                    }
                        else{

                            self.descriptors[0].state.set(EndpointState::Ctrl(CtrlState::WriteOut));

                            match client.ctrl_out(0, 0){
                                hil::usb::CtrlOutResult::Ok => {
                                    debug!("hil::usb::CtrlOutResult::Ok");
                                }
                                _ => {
                                    debug!("hil::usb::CtrlOutResult::Halted / Delay");
                                    self.ep_set_stall_out(0);
                                }
                            }
                        }
                    }
                }

            _err => {
                debug!("{:?}", _err);
                // An error occurred, we STALL
                debug!("Err -> Need to STALL");
                self.ep_set_stall_in(0);
                self.ep_set_stall_out(0);
            }

        }
    
        });
    }

    fn complete_ctrl_status(&self) {
        let endpoint = 0;

        self.client.map(|client| {
            client.ctrl_status(endpoint);
            client.ctrl_status_complete(endpoint);
            self.descriptors[endpoint]
                .state
                .set(EndpointState::Ctrl(CtrlState::Init));
        });
    }

    pub fn handle_interrupt(&self){

        if self.get_mode() == USB_DEVICE_MODE{

            // Mode Mismatch Interrupt
            if self.registers.gintsts.is_set(GINTSTS::MMIS) && self.registers.gintmsk.is_set(GINTMSK::MMISM){
                    debug!("MMIS");
                    /* incorrect mode, acknowledge the interrupt */
                    self.registers.gintsts.modify(GINTSTS::MMIS::SET);
            }

            //  Rx FIFO non-empty Interrupt
            if self.registers.gintsts.is_set(GINTSTS::RXFLVL) && self.registers.gintmsk.is_set(GINTMSK::RXFLVLM){
                    debug!("RXFLVL");

                    /* Mask the RXFLVLM interrupt */
                    self.registers.gintmsk.modify(GINTMSK::RXFLVLM::CLEAR);

                    let rx_fifo_pop = self.registers.grxstsp.get();

                    let ep_num = rx_fifo_pop & (0xF << 0);
                    let pktsts = (rx_fifo_pop & (0xF << 17)) >> 17;
                    let bcnt = (rx_fifo_pop & (0x7FF << 4)) >> 4;

                    // debug!("ep_num - {}", ep_num);
                    // debug!("pktsts - {}", pktsts);
                    // debug!("bcnt - {}", bcnt);

                    // It means that a SETUP transaction is completed
                    if bcnt == 0 && pktsts == STS_SETUP_COMPLETE{
                        debug!("Discarding end of SETUP packet");
                        self.receive_fifo[0].get();
                    }

                    if pktsts == STS_DATA_UPDT{
                        if bcnt != 0{
                                debug!("Received DATA packet.");                 
                                self.read_packet(ep_num as usize, bcnt);
                        }
                    }

                    else if pktsts == STS_SETUP_UPDT{
                            debug!("Received SETUP packet.");
                            self.read_packet(ep_num as usize, bcnt);
                    }

                    /* Unmask the interrupt */
                    self.registers.gintmsk.modify(GINTMSK::RXFLVLM::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::OEPINT) && self.registers.gintmsk.is_set(GINTMSK::OEPINT){
                    debug!("OEPINT");
                    
                    if self.registers.endpoint0_out_registers.doepint0.is_set(DOEPINTx::XFRC) && self.registers.doepmsk.is_set(DOEPMSK::XFRCM){
                        debug!("XFRC");
                        self.registers.endpoint0_out_registers.doepint0.modify(DOEPINTx::XFRC::CLEAR);
                        self.ep_out_transfer_complete_inter(0);
                    }

                    if self.registers.endpoint0_out_registers.doepint0.is_set(DOEPINTx::STUP) && self.registers.doepmsk.is_set(DOEPMSK::STUPM){
                        debug!("STUP");
                        self.registers.endpoint0_out_registers.doepint0.modify(DOEPINTx::STUP::CLEAR);
                        
                        self.ep_0_start_transfer_out();

                        let gSNPSiD = self.registers.cid.get() + 1;

                        if gSNPSiD > USB_OTG_CORE_ID_300A && self.registers.endpoint0_out_registers.doepint0.is_set(DOEPINTx::STSPHSRX){
                            debug!("gSNPSiD");
                            self.registers.endpoint0_out_registers.doepint0.modify(DOEPINTx::STSPHSRX::CLEAR);
                        }

                        self.handle_ctrl_setup();

                    }

                    if self.registers.endpoint0_out_registers.doepint0.is_set(DOEPINTx::OTEPDIS) && self.registers.doepmsk.is_set(DOEPMSK::OTEPDM){
                        debug!("OTEPDIS");
                        self.registers.endpoint0_out_registers.doepint0.modify(DOEPINTx::OTEPDIS::CLEAR);
                    }

                    if self.registers.endpoint0_out_registers.doepint0.is_set(DOEPINTx::STSPHSRX) && self.registers.doepmsk.is_set(DOEPMSK::STSPHSRXM){
                        debug!("STSPHSRX");
                        self.registers.endpoint0_out_registers.doepint0.modify(DOEPINTx::STSPHSRX::CLEAR);
                    }

                    if self.registers.endpoint0_out_registers.doepint0.is_set(DOEPINTx::NAK) && self.registers.doepmsk.is_set(DOEPMSK::NAKMSK){
                        debug!("NAK");
                        self.registers.endpoint0_out_registers.doepint0.modify(DOEPINTx::NAK::CLEAR);
                    }
            }

            if self.registers.gintsts.is_set(GINTSTS::IEPINT) && self.registers.gintmsk.is_set(GINTMSK::IEPINT){
                    debug!("IEPINT");
                    
                    if self.registers.endpoint0_in_registers.diepint0.is_set(DIEPINTx::XFRC) && self.registers.diepmsk.is_set(DIEPMSK::XFRCM){
                        debug!("XFRC");
                        self.registers.diepempmsk.modify(DIEPEMPMSK::INEPTXFEM::CLEAR);
                        self.registers.endpoint0_in_registers.diepint0.modify(DIEPINTx::XFRC::CLEAR);

                        self.ep_0_start_transfer_in();
                    }

                    if self.registers.endpoint0_in_registers.diepint0.is_set(DIEPINTx::TOC) && self.registers.diepmsk.is_set(DIEPMSK::TOM){
                        debug!("TOC");
                            self.registers.endpoint0_in_registers.diepint0.modify(DIEPINTx::TOC::CLEAR);
                    }

                    if self.registers.endpoint0_in_registers.diepint0.is_set(DIEPINTx::ITTXFE) && self.registers.diepmsk.is_set(DIEPMSK::ITTXFEMSK){
                        debug!("ITTXFE");
                        self.registers.endpoint0_in_registers.diepint0.modify(DIEPINTx::ITTXFE::CLEAR);
                    }

                    if self.registers.endpoint0_in_registers.diepint0.is_set(DIEPINTx::INEPNE) && self.registers.diepmsk.is_set(DIEPMSK::INEPNEM){
                        debug!("INEPNE");
                        self.registers.endpoint0_in_registers.diepint0.modify(DIEPINTx::INEPNE::CLEAR);
                    }

                    if self.registers.endpoint0_in_registers.diepint0.is_set(DIEPINTx::EPDISD) && self.registers.diepmsk.is_set(DIEPMSK::EPDM){
                        debug!("EPDISD");
                        self.registers.endpoint0_in_registers.diepint0.modify(DIEPINTx::EPDISD::CLEAR);
                    }
                    if self.registers.endpoint0_in_registers.diepint0.is_set(DIEPINTx::TXFE){
                        debug!("TXFE");
                            self.write_empty_tx_fifo(0);
                    }

            }

            if self.registers.gintsts.is_set(GINTSTS::WKUPINT) && self.registers.gintmsk.is_set(GINTMSK::WUIM){
                    debug!("WKUPINT");
                    self.registers.dctl.modify(DCTL::RWUSIG::CLEAR);

                    self.registers.gintsts.modify(GINTSTS::WKUPINT::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::USBSUSP) && self.registers.gintmsk.is_set(GINTMSK::USBSUSPM){
                    debug!("USBSUSP");

                    if self.registers.dsts.is_set(DSTS::SUSPSTS){
                        self.registers.pcgcctl.modify(PCGCCTL::STPPCLK::SET);
                    }

                    self.registers.gintsts.modify(GINTSTS::USBSUSP::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::LPMINT) && self.registers.gintmsk.is_set(GINTMSK::LPMINTM){
                    debug!("LPMINT");
                    self.registers.gintsts.modify(GINTSTS::LPMINT::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::USBRST) && self.registers.gintmsk.is_set(GINTMSK::USBRST){
                    debug!("USBRST");
                    
                    self.registers.dctl.modify(DCTL::RWUSIG::CLEAR);

                    self.flush_tx_fifo(10);

                    self.registers.endpoint0_in_registers.diepint0.set(0xFB7F as u32);
                    self.registers.endpoint0_in_registers.diepctl0.modify(DIEPCTL0::STALL::CLEAR + 
                                                                          DIEPCTL0::SNAK::SET);

                    self.registers.endpoint0_out_registers.doepint0.set(0xFB7F as u32);
                    self.registers.endpoint0_out_registers.doepctl0.modify(DOEPCTL0::STALL::CLEAR +
                                                                           DOEPCTL0::SNAK::SET);

                    for i in 1..NUM_ENDPOINTS{

                        self.registers.endpoint_in_registers[(i-1) as usize].diepintx.set(0xFB7F as u32);
                        self.registers.endpoint_in_registers[(i-1) as usize].diepctlx.modify(DIEPCTLx::STALL::CLEAR +
                                                                                             DIEPCTLx::SNAK::SET);

                        self.registers.endpoint_out_registers[(i-1) as usize].doepintx.set(0xFB7F as u32);
                        self.registers.endpoint_out_registers[(i-1) as usize].doepctlx.modify(DOEPCTLx::STALL::CLEAR +
                                                                                              DOEPCTLx::SNAK::SET);

                    }

                    self.registers.daintmsk.modify(DAINTMSK::OEPM.val(1) +
                                                   DAINTMSK::IEPM.val(1));

                    self.registers.doepmsk.modify(DOEPMSK::STUPM::SET +
                                                  DOEPMSK::XFRCM::SET +
                                                  DOEPMSK::EPDM::SET +
                                                  DOEPMSK::STSPHSRXM::SET +
                                                  DOEPMSK::NAKMSK::SET
                                                );

                    self.registers.diepmsk.modify(DIEPMSK::TOM::SET +
                                                  DIEPMSK::XFRCM::SET +
                                                  DIEPMSK::EPDM::SET
                                                );

                    self.registers.dcfg.modify(DCFG::DAD.val(0));

                    self.ep0_out_start();

                    self.client.map(|client| {
                        client.bus_reset();
                    });
                    
                    self.registers.gintsts.modify(GINTSTS::USBRST::SET);

            }

            if self.registers.gintsts.is_set(GINTSTS::ENUMDNE) && self.registers.gintmsk.is_set(GINTMSK::ENUMDNEM){
                    debug!("ENUMDNE");

                    self.activate_setup();
                    self.set_turnaround_time();
                    self.usb_reset();

                    self.ep0_out_start();

                    self.registers.gintsts.modify(GINTSTS::ENUMDNE::SET);


            }

            if self.registers.gintsts.is_set(GINTSTS::SOF) && self.registers.gintmsk.is_set(GINTMSK::SOFM){
                    debug!("SOF");
                    self.registers.gintsts.modify(GINTSTS::SOF::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::IISOIXFR) && self.registers.gintmsk.is_set(GINTMSK::IISOIXFRM){
                    debug!("IISOIXFR");
                    self.registers.gintsts.modify(GINTSTS::IISOIXFR::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::INCOMPISOOUT) && self.registers.gintmsk.is_set(GINTMSK::IISOOXFRM){
                    debug!("INCOMPISOOUT");
                    self.registers.gintsts.modify(GINTSTS::INCOMPISOOUT::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::SRQINT) && self.registers.gintmsk.is_set(GINTMSK::SRQIM){
                    debug!("SRQINT");
                    self.registers.gintsts.modify(GINTSTS::SRQINT::SET);
            }

            if self.registers.gintsts.is_set(GINTSTS::OTGINT) && self.registers.gintmsk.is_set(GINTMSK::OTGINT){
                    debug!("OTGINT");
                    self.registers.gintsts.modify(GINTSTS::OTGINT::SET);
            }

        }
    }
    
    fn write_empty_tx_fifo(&self, epnum: u32){
        let size = self.descriptors[0].transfer_size.get();
        self.write_packet(epnum as usize, size as usize);
    }

    fn ep_out_transfer_complete_inter(&self, epnum: u32){
        let _i = epnum;
    }

    fn ep_out_setup_packet_inter(&self, epnum: u32){
        let _i = epnum;
    }

    pub fn initialization(&self){

        self.disable_global_interrupt();

        self.core_initialization();

        self.set_current_mode(USB_DEVICE_MODE);

        self.device_initialization();

        self.device_disconnect();
        
        self.set_rx_fifo(0x80 as u32);
        self.set_tx_fifo(0, 0x40 as u32);
        self.set_tx_fifo(1, 0x80 as u32);
    }

    fn device_start(&self){
        self.enable_global_interrupt();
        self.device_connect();
    }

    fn set_tx_fifo(&self, tx_fifo_no: u32, size: u32){

        let mut tx_offset: u32 = self.registers.grxfsiz.read(GRXFSIZ::RXFD);

        if tx_fifo_no == 0{
                self.registers.dieptxf0.modify(DIEPTXF0::TX0FD.val(size) +
                                               DIEPTXF0::TX0FSA.val(tx_offset));
        }

        else{
                tx_offset += self.registers.dieptxf0.read(DIEPTXF0::TX0FD);
                for i in 0..tx_fifo_no - 1{
                        tx_offset += self.registers.dieptxfx[i as usize].read(DIEPTXFx::INEPTXFD);
                }
                self.registers.dieptxfx[(tx_fifo_no - 1) as usize].modify(DIEPTXFx::INEPTXFD.val(size) +
                                                                          DIEPTXFx::INEPTXSA.val(tx_offset));
        }

    }

    fn set_rx_fifo(&self, size: u32){
        self.registers.grxfsiz.set(size);
    }   
    // Powers the USB PHY on
    fn enable(&self) {
        if self.get_state() != UsbState::Disabled {
            debug!("USB is already enabled");
            return;
        }
        self.initialization();
        self.state.set(UsbState::Initialized);
    }

    // Allows the peripheral to be enumerated by the USB master
    fn start(&self) {
        self.device_start();
        self.state.set(UsbState::Started);
    }
}

impl<'a> hil::usb::UsbController<'a> for Usbotg<'a> {
    fn set_client(&self, client: &'a dyn hil::usb::Client<'a>) {
        //debug!("set_client - UsbState = {:?}", self.get_state());
        self.client.set(client);
    }

    fn endpoint_set_ctrl_buffer(&self, buf: &'a [VolatileCell<u8>]) {
        //debug!("ep_set_ctrl_buffer");
        if buf.len() < 8 {
            panic!("Endpoint buffer must be at least 8 bytes");
        }
        if !buf.len().is_power_of_two() {
            panic!("Buffer size must be a power of 2");
        }

        self.descriptors[0].slice_in.set(buf);
        self.descriptors[0].slice_out.set(buf);
    }

    fn endpoint_set_in_buffer(&self, endpoint: usize, buf: &'a [VolatileCell<u8>]) {
        //debug!("ep_set_in_buffer");
        if buf.len() < 8 {
            panic!("Endpoint buffer must be at least 8 bytes");
        }
        if !buf.len().is_power_of_two() {
            panic!("Buffer size must be a power of 2");
        }
        if endpoint == 0 || endpoint >= NUM_ENDPOINTS {
            panic!("Endpoint number is invalid");
        }
        self.descriptors[endpoint].slice_in.set(buf);
    }

    fn endpoint_set_out_buffer(&self, endpoint: usize, buf: &'a [VolatileCell<u8>]) {
        //debug!("ep_set_out_buffer");
        if buf.len() < 8 {
            panic!("Endpoint buffer must be at least 8 bytes");
        }
        if !buf.len().is_power_of_two() {
            panic!("Buffer size must be a power of 2");
        }
        if endpoint == 0 || endpoint >= NUM_ENDPOINTS {
            panic!("Endpoint number is invalid");
        }
        self.descriptors[endpoint].slice_out.set(buf);
    }

    fn enable_as_device(&self, speed: hil::usb::DeviceSpeed) {
        //debug!("enable_as_device - Usbstate = {:?}", self.get_state());
        match speed {
            hil::usb::DeviceSpeed::Low => internal_err!("Low speed is not supported"),
            hil::usb::DeviceSpeed::Full => {},
        }
        self.enable();
    }

    fn attach(&self) {
        //debug!("attach() - Usbstate = {:?}", self.get_state());
        self.start();
    }

    fn detach(&self) {
        unimplemented!()
    }

    fn set_address(&self, addr: u16) {
        //debug!("set_address - {}", addr);
        self.addr.set(addr);
        self.set_device_address(self.addr.get() as u32);
    }

    fn enable_address(&self) {
        //unimplemented!();
        //debug!("enable_address");
    }

    fn endpoint_in_enable(&self, transfer_type: TransferType, endpoint: usize) {
        //debug!("ep_in_enable - {}  Transfer: {:?}", endpoint, transfer_type);
       
        self.activate_endpoint_in(transfer_type, endpoint);

    }
    
    fn endpoint_out_enable(&self, transfer_type: TransferType, endpoint: usize) {
        //debug!("ep_out_enable - {}  Transfer: {:?}", endpoint, transfer_type);

        self.activate_endpoint_out(transfer_type, endpoint);
    }

    fn endpoint_in_out_enable(&self, transfer_type: TransferType, endpoint: usize) {
        //debug!("ep_in_out_enable - {}  Transfer: {:?}", endpoint, transfer_type);
        
        self.endpoint_in_enable(transfer_type, endpoint);
        self.endpoint_out_enable(transfer_type, endpoint);

    }

    fn endpoint_resume_in(&self, endpoint: usize) {
        let _e = endpoint;
        debug!("endpoint_resume_in - {}", endpoint);
        //unimplemented!();
    }

    fn endpoint_resume_out(&self, endpoint: usize) {
        let _e = endpoint;
        debug!("endpoint_resume_out - {}", endpoint);
        //unimplemented!();
    }


}

struct USBOTGClock<'a>(rcc::PeripheralClock<'a>);

impl ClockInterface for USBOTGClock<'_> {
    fn is_enabled(&self) -> bool {
        self.0.is_enabled()
    }

    fn enable(&self) {
        self.0.enable();
    }

    fn disable(&self) {
        self.0.disable();
    }
}
