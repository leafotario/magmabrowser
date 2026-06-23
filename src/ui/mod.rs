pub mod font;
pub mod omnibox;
pub mod settings;

pub const TABBAR_HEIGHT: u32 = 32;
pub const OMNIBOX_HEIGHT: u32 = 32;
pub const CHROME_HEIGHT: u32 = TABBAR_HEIGHT + OMNIBOX_HEIGHT;

use crate::fsm::tab_manager::Tab;

/// Limpa o buffer com uma cor de fundo
pub fn clear_rect(buffer: &mut [u32], buf_width: usize, x: usize, y: usize, w: usize, h: usize, color: u32) {
    for row in y..(y + h) {
        let offset = row * buf_width + x;
        for col in 0..w {
            if offset + col < buffer.len() {
                buffer[offset + col] = color;
            }
        }
    }
}

/// Desenha um caractere 8x16 usando a fonte bitmap
pub fn draw_char(buffer: &mut [u32], buf_width: usize, x: usize, y: usize, c: char, color: u32) {
    let mut code = c as usize;
    if code >= 128 {
        code = '?' as usize;
    }
    let glyph = &font::FONT_8X16[code];
    for (row_idx, row_val) in glyph.iter().enumerate() {
        let py = y + row_idx;
        let offset = py * buf_width + x;
        for col_idx in 0..8 {
            if (row_val & (1 << (7 - col_idx))) != 0 {
                let px = offset + col_idx;
                if px < buffer.len() {
                    buffer[px] = color;
                }
            }
        }
    }
}

/// Desenha uma string com a fonte 8x16, limitando a largura máxima
pub fn draw_string(buffer: &mut [u32], buf_width: usize, x: usize, y: usize, text: &str, color: u32, max_width: usize) {
    let mut current_x = x;
    let char_width = 8;
    for c in text.chars() {
        if current_x + char_width > x + max_width {
            // Truncamento (desenhar reticências seria legal, mas vamos apenas cortar por simplicidade)
            if current_x >= 16 {
                draw_char(buffer, buf_width, current_x - 16, y, '.', color);
                draw_char(buffer, buf_width, current_x - 8, y, '.', color);
            }
            break;
        }
        draw_char(buffer, buf_width, current_x, y, c, color);
        current_x += char_width;
    }
}

/// Renderiza a barra de abas completa no buffer
pub fn render_tab_bar(buffer: &mut [u32], width: usize, tabs: &[Tab], active_index: usize) {
    let bg_color = 0xFF_1E_1E_1E; // Fundo escuro nativo
    let fg_color = 0xFF_D4_D4_D4; // Texto claro
    let active_bg = 0xFF_3C_3C_3C; // Fundo aba ativa
    let inactive_bg = 0xFF_25_25_25; // Fundo aba inativa
    let border_color = 0xFF_00_00_00; // Divisórias
    
    // Limpar o fundo da Tab Bar
    clear_rect(buffer, width, 0, 0, width, TABBAR_HEIGHT as usize, bg_color);
    
    let tab_width = 200;
    
    for (i, tab) in tabs.iter().enumerate() {
        let start_x = i * tab_width;
        if start_x >= width { break; } // Não renderiza abas fora da tela
        
        let is_active = i == active_index;
        let t_bg = if is_active { active_bg } else { inactive_bg };
        
        let w = if start_x + tab_width > width { width - start_x } else { tab_width };
        
        // Fundo da aba
        clear_rect(buffer, width, start_x, 0, w, TABBAR_HEIGHT as usize, t_bg);
        
        // Borda direita
        clear_rect(buffer, width, start_x + w - 1, 0, 1, TABBAR_HEIGHT as usize, border_color);
        
        // Ícone minúsculo / Margem e Título
        let text_x = start_x + 10;
        let text_y = (TABBAR_HEIGHT as usize - 16) / 2; // Centralizado
        
        let title_to_draw = if tab.title.is_empty() { "Carregando..." } else { &tab.title };
        
        draw_string(buffer, width, text_x, text_y, title_to_draw, fg_color, w.saturating_sub(30));
        
        // Botão X
        let close_x = start_x + w - 20;
        if close_x > start_x + 20 {
            draw_char(buffer, width, close_x, text_y, 'x', 0xFF_AA_AA_AA);
        }
        
        // Linha superior de destaque na aba ativa
        if is_active {
            clear_rect(buffer, width, start_x, 0, w, 2, 0xFF_00_7A_CC); // Azul
        }
    }
    
    // Botão nova aba '+'
    let plus_x = tabs.len() * tab_width + 10;
    if plus_x < width {
        draw_char(buffer, width, plus_x, (TABBAR_HEIGHT as usize - 16) / 2, '+', fg_color);
    }
}
