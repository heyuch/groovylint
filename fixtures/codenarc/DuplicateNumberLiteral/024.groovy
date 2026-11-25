
            package com.example

            enum BasicEnum{
                AA(318L),
                AB(319L),
                AC(320L),

                AP(3L)  // There will also be a 3 used as an internal generated index

                final Long id

                private BasicEnum(Long id) {
                    this.id = id
                }
            }
            