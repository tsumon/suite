use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::NetworkManagement::IpHelper::{
    FreeMibTable, GetIfEntry2, GetIfTable2, MIB_IF_ROW2, MIB_IF_TABLE2,
};
use windows::Win32::NetworkManagement::Ndis::{IfOperStatusUp, MediaConnectStateConnected};

use crate::types::{InterfaceSnapshot, InterfaceTable};

pub struct IpHelperInterfaceTable;

impl IpHelperInterfaceTable {
    fn wide_slice_to_string(slice: &[u16]) -> String {
        let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
        String::from_utf16_lossy(&slice[..len])
    }

    fn read_row(row: &MIB_IF_ROW2) -> InterfaceSnapshot {
        InterfaceSnapshot {
            if_index: row.InterfaceIndex,
            alias: Self::wide_slice_to_string(&row.Alias),
            description: Self::wide_slice_to_string(&row.Description),
            is_up: row.OperStatus == IfOperStatusUp,
            if_type: row.Type,
            media_connected: row.MediaConnectState == MediaConnectStateConnected,
            in_octets: row.InOctets,
            out_octets: row.OutOctets,
        }
    }
}

impl InterfaceTable for IpHelperInterfaceTable {
    fn get_table(&self) -> Result<Vec<InterfaceSnapshot>, String> {
        let mut table_ptr: *mut MIB_IF_TABLE2 = std::ptr::null_mut();
        let status = unsafe { GetIfTable2(&mut table_ptr) };
        if status != ERROR_SUCCESS || table_ptr.is_null() {
            return Err(format!("GetIfTable2 failed with status {}.", status.0));
        }

        let count = unsafe { (*table_ptr).NumEntries as usize };
        let rows = unsafe { std::slice::from_raw_parts((*table_ptr).Table.as_ptr(), count) };
        let list = rows.iter().map(Self::read_row).collect();
        unsafe {
            FreeMibTable(table_ptr.cast());
        }
        Ok(list)
    }

    fn get_entry(&self, if_index: u32) -> Option<InterfaceSnapshot> {
        let mut row = MIB_IF_ROW2 {
            InterfaceIndex: if_index,
            ..Default::default()
        };
        let status = unsafe { GetIfEntry2(&mut row) };
        if status != ERROR_SUCCESS {
            return None;
        }
        Some(Self::read_row(&row))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_get_if_table2_succeeds() {
        let table = IpHelperInterfaceTable.get_table().expect("GetIfTable2 should succeed on Windows");
        assert!(!table.is_empty());
    }
}
