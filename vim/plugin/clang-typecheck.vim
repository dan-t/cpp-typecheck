command! -nargs=* -complete=file ClangTypecheckFile call s:ClangTypecheckFile(<f-args>)

function! s:ClangTypecheckFile(...)
   let l:old_makeprg = &makeprg

   let l:cmd = 'clang-typecheck ' . join(a:000)
   let &makeprg = l:cmd
   silent update
   silent lmake

   let &makeprg = l:old_makeprg
endfunction
