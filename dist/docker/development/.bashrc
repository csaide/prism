# Some useful list aliases
alias ll="ls -al"
alias l="ls -l"

# Some git aliases
alias commit="git commit -S"
alias add="git add"
alias push="git push -u origin"
alias checkout="git checkout"

# Make sure that the gpg agent knows where to
# ask for passwords.
export GPG_TTY=$(tty)

# Make sure we get completion.
source /etc/bash_completion
