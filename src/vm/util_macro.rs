#[macro_export]
macro_rules! slot_at_index {
    ($self: expr, $idx: expr) => {
        $self.stack[current_frame!($self).slot_bottom + $idx]
    };
}

#[macro_export]
macro_rules! current_frame_slot {
    ($self: expr) => {
        $self.stack[current_frame!($self).slot_bottom + current_frame!($self).slot_count]
    };
}

#[macro_export]
macro_rules! current_frame {
    ($self: expr) => {
        $self.frames.last_mut().unwrap()
    };
}

#[macro_export]
macro_rules! read_byte {
    ($self: expr) => {{
        current_frame!($self).ip = unsafe { current_frame!($self).ip.add(1) };
        unsafe { *current_frame!($self).ip }
    }};
}

#[macro_export]
macro_rules! read_2bytes {
    ($self: expr) => {
        from_little_endian([read_byte!($self), read_byte!($self)])
    };
}

#[macro_export]
macro_rules! read_4bytes {
    ($self: expr) => {
        from_little_endian_u32([
            read_byte!($self),
            read_byte!($self),
            read_byte!($self),
            read_byte!($self),
        ])
    };
}

#[macro_export]
macro_rules! try_push {
    ($self: expr, $val: expr) => {
        match $val {
            Ok(v) => $self.push(v),
            Err(msg) => $self.runtime_err(msg),
        }
    };
}

#[macro_export]
macro_rules! binary_op {
    ($self: expr, $op: tt) => {
        let right = $self.pop();
        let left = $self.pop();
        try_push!($self, left $op right);
    };
}

#[macro_export]
macro_rules! partial_match {
    ($self: expr, $target: expr, $jump: expr, $cond: expr, $type: ty, $rn: expr) => {
        if as_t!($target, FEmpty).is_some() {
            {} // do nothing
        } else if let Some(target) = as_t!($target, $type) {
            if target.0 != $cond.0 {
                $self.jump($jump);
                *$rn = true;
            }
        } else if as_t!($target, FVar).is_some() {
            // TODO: fix this later
        } else {
            $self.jump($jump);
            *$rn = true;
        }
    };
}
