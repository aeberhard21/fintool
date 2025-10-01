/* ------------------------------------------------------------------------
    Copyright (C) 2025  Andrew J. Eberhard

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
  -----------------------------------------------------------------------*/
  pub fn get_gnu_gpl_conditions() -> String { 
    return "This program is free software: you can redistribute it and/or modify 
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.".to_string();
  }

  pub fn get_gnu_gpl_warranty() -> String { 
    return "This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.".to_string();
  }

  pub fn license_banner() -> String { 
    return "FINTOOL  Copyright (C) 2025  Andrew J. Eberhard
This program comes with ABSOLUTELY NO WARRANTY; for details select `Show Warranty'.
This is free software, and you are welcome to redistribute it
under certain conditions; select `Show Conditions' for details.".to_string();
  }